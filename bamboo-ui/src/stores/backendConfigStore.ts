import { create } from "zustand";
import { persist } from "zustand/middleware";

import type {
  ApiError,
  AgentConfig,
  BackendConfig,
  GatewayConfig,
  LlmProviderConfig,
  LoggingConfig,
  ServerConfig,
  SkillsConfig,
  StorageConfig,
} from "@/types";
import { getBackendConfig, saveBackendConfig, toApiError } from "@/lib/api";

export interface BackendConfigErrors {
  version?: string;
  server?: Partial<Record<keyof ServerConfig, string>>;
  gateway?: Partial<Record<keyof GatewayConfig, string>>;
  llm?: {
    default_provider?: string;
    providers?: Record<string, Partial<Record<keyof LlmProviderConfig, string>>>;
  };
  skills?: Partial<Record<keyof SkillsConfig, string | string[]>>;
  agent?: Partial<Record<keyof AgentConfig, string>>;
  storage?: Partial<Record<keyof StorageConfig, string>>;
  logging?: Partial<Record<keyof LoggingConfig, string>>;
}

interface BackendConfigValidationResult {
  isValid: boolean;
  errors: BackendConfigErrors;
}

const defaultBackendConfig: BackendConfig = {
  version: "0.1.0",
  server: {
    port: 8081,
    host: "127.0.0.1",
    cors: true,
  },
  gateway: {
    enabled: true,
    bind: "127.0.0.1:18790",
    auth_token: null,
    max_connections: 100,
    heartbeat_interval_secs: 30,
  },
  llm: {
    default_provider: "local",
    providers: {
      local: {
        enabled: true,
        base_url: "http://localhost:12123",
        model: "kimi-for-coding",
        auth_type: "bearer",
        env: "LOCAL_API_KEY",
        timeout_seconds: 60,
      },
    },
  },
  skills: {
    enabled: true,
    auto_reload: true,
    directories: ["~/.bamboo/skills"],
  },
  agent: {
    max_rounds: 10,
    system_prompt: "You are a helpful assistant",
    timeout_seconds: 300,
  },
  storage: {
    type: "jsonl",
    path: "~/.bamboo/sessions",
  },
  logging: {
    level: "info",
    file: "~/.bamboo/logs/bamboo.log",
    max_size_mb: 100,
    max_files: 5,
  },
};

const createProviderDefaults = (): LlmProviderConfig => ({
  enabled: true,
  base_url: "",
  model: "",
  auth_type: "bearer",
  env: "",
  timeout_seconds: 60,
});

const isBlank = (value: string | null | undefined) => !value || value.trim().length === 0;

export const validateBackendConfig = (config: BackendConfig): BackendConfigValidationResult => {
  const errors: BackendConfigErrors = {};
  let isValid = true;

  const setError = (setter: () => void) => {
    isValid = false;
    setter();
  };

  if (isBlank(config.version)) {
    setError(() => {
      errors.version = "版本不能为空";
    });
  }

  if (!Number.isFinite(config.server.port) || config.server.port < 1 || config.server.port > 65535) {
    setError(() => {
      errors.server = { ...errors.server, port: "端口范围应为 1-65535" };
    });
  }

  if (isBlank(config.server.host)) {
    setError(() => {
      errors.server = { ...errors.server, host: "主机地址不能为空" };
    });
  }

  if (isBlank(config.gateway.bind)) {
    setError(() => {
      errors.gateway = { ...errors.gateway, bind: "绑定地址不能为空" };
    });
  }

  if (!Number.isFinite(config.gateway.max_connections) || config.gateway.max_connections < 1) {
    setError(() => {
      errors.gateway = {
        ...errors.gateway,
        max_connections: "最大连接数必须大于 0",
      };
    });
  }

  if (!Number.isFinite(config.gateway.heartbeat_interval_secs) || config.gateway.heartbeat_interval_secs < 1) {
    setError(() => {
      errors.gateway = {
        ...errors.gateway,
        heartbeat_interval_secs: "心跳间隔必须大于 0",
      };
    });
  }

  if (isBlank(config.llm.default_provider)) {
    setError(() => {
      errors.llm = { ...errors.llm, default_provider: "默认 provider 不能为空" };
    });
  }

  const providerKeys = Object.keys(config.llm.providers || {});
  if (providerKeys.length === 0) {
    setError(() => {
      errors.llm = { ...errors.llm, default_provider: "至少需要一个 provider" };
    });
  }

  if (config.llm.default_provider && !config.llm.providers[config.llm.default_provider]) {
    setError(() => {
      errors.llm = {
        ...errors.llm,
        default_provider: "默认 provider 必须存在于 providers 列表中",
      };
    });
  }

  providerKeys.forEach((key) => {
    const provider = config.llm.providers[key];
    if (!provider) return;
    const providerErrors: Partial<Record<keyof LlmProviderConfig, string>> = {};

    if (isBlank(provider.base_url)) {
      providerErrors.base_url = "Base URL 不能为空";
    }
    if (isBlank(provider.model)) {
      providerErrors.model = "模型名称不能为空";
    }
    if (isBlank(provider.auth_type)) {
      providerErrors.auth_type = "认证类型不能为空";
    }
    if (isBlank(provider.env)) {
      providerErrors.env = "环境变量不能为空";
    }
    if (!Number.isFinite(provider.timeout_seconds) || provider.timeout_seconds < 1) {
      providerErrors.timeout_seconds = "超时必须大于 0 秒";
    }

    if (Object.keys(providerErrors).length > 0) {
      setError(() => {
        errors.llm = {
          ...errors.llm,
          providers: {
            ...(errors.llm?.providers ?? {}),
            [key]: providerErrors,
          },
        };
      });
    }
  });

  if (config.skills.directories.length === 0) {
    setError(() => {
      errors.skills = { ...errors.skills, directories: ["至少需要一个目录"] };
    });
  }

  config.skills.directories.forEach((dir, index) => {
    if (isBlank(dir)) {
      setError(() => {
        const currentDirs = (errors.skills?.directories as string[] | undefined) ?? [];
        currentDirs[index] = "目录不能为空";
        errors.skills = {
          ...errors.skills,
          directories: currentDirs,
        };
      });
    }
  });

  if (!Number.isFinite(config.agent.max_rounds) || config.agent.max_rounds < 1) {
    setError(() => {
      errors.agent = { ...errors.agent, max_rounds: "最大轮次必须大于 0" };
    });
  }

  if (isBlank(config.agent.system_prompt)) {
    setError(() => {
      errors.agent = { ...errors.agent, system_prompt: "系统提示不能为空" };
    });
  }

  if (!Number.isFinite(config.agent.timeout_seconds) || config.agent.timeout_seconds < 1) {
    setError(() => {
      errors.agent = { ...errors.agent, timeout_seconds: "超时必须大于 0 秒" };
    });
  }

  if (isBlank(config.storage.type)) {
    setError(() => {
      errors.storage = { ...errors.storage, type: "存储类型不能为空" };
    });
  }

  if (isBlank(config.storage.path)) {
    setError(() => {
      errors.storage = { ...errors.storage, path: "存储路径不能为空" };
    });
  }

  if (isBlank(config.logging.level)) {
    setError(() => {
      errors.logging = { ...errors.logging, level: "日志级别不能为空" };
    });
  }

  if (isBlank(config.logging.file)) {
    setError(() => {
      errors.logging = { ...errors.logging, file: "日志文件路径不能为空" };
    });
  }

  if (!Number.isFinite(config.logging.max_size_mb) || config.logging.max_size_mb < 1) {
    setError(() => {
      errors.logging = { ...errors.logging, max_size_mb: "日志大小必须大于 0" };
    });
  }

  if (!Number.isFinite(config.logging.max_files) || config.logging.max_files < 1) {
    setError(() => {
      errors.logging = { ...errors.logging, max_files: "日志文件数量必须大于 0" };
    });
  }

  return { isValid, errors };
};

const applyValidation = (config: BackendConfig) => {
  const validation = validateBackendConfig(config);
  return {
    config,
    validationErrors: validation.errors,
    isValid: validation.isValid,
  };
};

export interface BackendConfigState {
  config: BackendConfig;
  validationErrors: BackendConfigErrors;
  isValid: boolean;
  loading: boolean;
  saving: boolean;
  error?: string;
  hasLoaded: boolean;
  lastLoadedAt?: string;
  lastSavedAt?: string;
  setConfig: (config: BackendConfig) => void;
  updateConfig: (patch: Partial<BackendConfig>) => void;
  updateServer: (patch: Partial<ServerConfig>) => void;
  updateGateway: (patch: Partial<GatewayConfig>) => void;
  updateSkills: (patch: Partial<SkillsConfig>) => void;
  updateAgent: (patch: Partial<AgentConfig>) => void;
  updateStorage: (patch: Partial<StorageConfig>) => void;
  updateLogging: (patch: Partial<LoggingConfig>) => void;
  updateVersion: (version: string) => void;
  addProvider: (key: string) => void;
  updateProvider: (key: string, patch: Partial<LlmProviderConfig>) => void;
  removeProvider: (key: string) => void;
  updateDefaultProvider: (key: string) => void;
  addSkillsDirectory: (path: string) => void;
  updateSkillsDirectory: (index: number, path: string) => void;
  removeSkillsDirectory: (index: number) => void;
  loadConfig: () => Promise<{ success: boolean; error?: ApiError }>;
  saveConfig: () => Promise<{ success: boolean; error?: ApiError }>;
  resetConfig: () => void;
}

export const useBackendConfigStore = create<BackendConfigState>()(
  persist(
    (set, get) => ({
      ...applyValidation(defaultBackendConfig),
      loading: false,
      saving: false,
      hasLoaded: false,
      setConfig: (config) => {
        set(() => applyValidation(config));
      },
      updateConfig: (patch) => {
        set((state) => applyValidation({ ...state.config, ...patch }));
      },
      updateVersion: (version) => {
        set((state) => applyValidation({ ...state.config, version }));
      },
      updateServer: (patch) => {
        set((state) =>
          applyValidation({
            ...state.config,
            server: { ...state.config.server, ...patch },
          })
        );
      },
      updateGateway: (patch) => {
        set((state) =>
          applyValidation({
            ...state.config,
            gateway: { ...state.config.gateway, ...patch },
          })
        );
      },
      updateSkills: (patch) => {
        set((state) =>
          applyValidation({
            ...state.config,
            skills: { ...state.config.skills, ...patch },
          })
        );
      },
      updateAgent: (patch) => {
        set((state) =>
          applyValidation({
            ...state.config,
            agent: { ...state.config.agent, ...patch },
          })
        );
      },
      updateStorage: (patch) => {
        set((state) =>
          applyValidation({
            ...state.config,
            storage: { ...state.config.storage, ...patch },
          })
        );
      },
      updateLogging: (patch) => {
        set((state) =>
          applyValidation({
            ...state.config,
            logging: { ...state.config.logging, ...patch },
          })
        );
      },
      addProvider: (key) => {
        const trimmedKey = key.trim();
        if (!trimmedKey) return;
        set((state) => {
          if (state.config.llm.providers[trimmedKey]) {
            return state;
          }
          return applyValidation({
            ...state.config,
            llm: {
              ...state.config.llm,
              providers: {
                ...state.config.llm.providers,
                [trimmedKey]: createProviderDefaults(),
              },
            },
          });
        });
      },
      updateProvider: (key, patch) => {
        set((state) => {
          const current = state.config.llm.providers[key];
          if (!current) return state;
          return applyValidation({
            ...state.config,
            llm: {
              ...state.config.llm,
              providers: {
                ...state.config.llm.providers,
                [key]: { ...current, ...patch },
              },
            },
          });
        });
      },
      removeProvider: (key) => {
        set((state) => {
          const { [key]: removed, ...rest } = state.config.llm.providers;
          const nextDefault =
            state.config.llm.default_provider === key
              ? Object.keys(rest)[0] ?? ""
              : state.config.llm.default_provider;
          return applyValidation({
            ...state.config,
            llm: {
              ...state.config.llm,
              default_provider: nextDefault,
              providers: rest,
            },
          });
        });
      },
      updateDefaultProvider: (key) => {
        set((state) =>
          applyValidation({
            ...state.config,
            llm: {
              ...state.config.llm,
              default_provider: key,
            },
          })
        );
      },
      addSkillsDirectory: (path) => {
        const trimmedPath = path.trim();
        if (!trimmedPath) return;
        set((state) =>
          applyValidation({
            ...state.config,
            skills: {
              ...state.config.skills,
              directories: [...state.config.skills.directories, trimmedPath],
            },
          })
        );
      },
      updateSkillsDirectory: (index, path) => {
        set((state) =>
          applyValidation({
            ...state.config,
            skills: {
              ...state.config.skills,
              directories: state.config.skills.directories.map((dir, i) =>
                i === index ? path : dir
              ),
            },
          })
        );
      },
      removeSkillsDirectory: (index) => {
        set((state) =>
          applyValidation({
            ...state.config,
            skills: {
              ...state.config.skills,
              directories: state.config.skills.directories.filter((_, i) => i !== index),
            },
          })
        );
      },
      loadConfig: async () => {
        set({ loading: true, error: undefined });
        try {
          const data = await getBackendConfig();
          set({
            ...applyValidation(data),
            loading: false,
            hasLoaded: true,
            lastLoadedAt: new Date().toISOString(),
          });
          return { success: true };
        } catch (error) {
          const apiError = toApiError(error);
          set({
            loading: false,
            error: apiError.message,
            hasLoaded: true,
          });
          return { success: false, error: apiError };
        }
      },
      saveConfig: async () => {
        const { config } = get();
        const validation = validateBackendConfig(config);
        if (!validation.isValid) {
          set({
            validationErrors: validation.errors,
            isValid: false,
          });
          return {
            success: false,
            error: {
              message: "配置校验失败，请修正后再保存",
            },
          };
        }

        set({ saving: true, error: undefined });
        try {
          const saved = await saveBackendConfig(config);
          set({
            ...applyValidation(saved),
            saving: false,
            lastSavedAt: new Date().toISOString(),
          });
          return { success: true };
        } catch (error) {
          const apiError = toApiError(error);
          set({ saving: false, error: apiError.message });
          return { success: false, error: apiError };
        }
      },
      resetConfig: () => {
        set(() => applyValidation(defaultBackendConfig));
      },
    }),
    {
      name: "bamboo-backend-config",
      partialize: (state) => ({
        config: state.config,
        lastLoadedAt: state.lastLoadedAt,
        lastSavedAt: state.lastSavedAt,
      }),
    }
  )
);
