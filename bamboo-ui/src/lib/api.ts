import axios, { AxiosError, type AxiosInstance, type AxiosRequestConfig, type InternalAxiosRequestConfig } from "axios";

import type { ApiError, BackendConfig, Config, MaskingConfig, MaskingTestRequest, MaskingTestResponse, SystemPrompt, PromptListResponse, PromptResponse, MemoryListResponse, SessionMemoryResponse } from "@/types";
import { useConfigStore } from "@/stores/configStore";

const DEFAULT_TIMEOUT = 30_000;

const createRequestId = () => {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }

  return `req_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
};

export const toApiError = (error: unknown): ApiError => {
  if (axios.isAxiosError(error)) {
    const axiosError = error as AxiosError<unknown>;
    const status = axiosError.response?.status;
    const message =
      (axiosError.response?.data as { message?: string } | undefined)?.message ||
      axiosError.message ||
      "Request failed";
    return {
      message,
      status,
      code: axiosError.code,
      details: axiosError.response?.data ?? axiosError.toJSON?.(),
    };
  }

  if (error instanceof Error) {
    return { message: error.message, details: error };
  }

  return { message: "Unknown error", details: error };
};

const applyConfig = (config: InternalAxiosRequestConfig, storeConfig: Config): InternalAxiosRequestConfig => {
  if (storeConfig.apiUrl) {
    config.baseURL = storeConfig.apiUrl;
  }

  config.timeout = config.timeout ?? DEFAULT_TIMEOUT;
  const headers = (config.headers ?? {}) as Record<string, string>;
  headers["Accept"] = headers["Accept"] ?? "application/json";

  if (!headers["Content-Type"] && config.method !== "get") {
    headers["Content-Type"] = "application/json";
  }

  headers["x-request-id"] = headers["x-request-id"] ?? createRequestId();

  if (storeConfig.apiKey) {
    headers["Authorization"] = `Bearer ${storeConfig.apiKey}`;
  }

  config.headers = headers as unknown as InternalAxiosRequestConfig['headers'];

  return config;
};

const apiClient: AxiosInstance = axios.create();

apiClient.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    const storeConfig = useConfigStore.getState().config;
    return applyConfig(config, storeConfig);
  },
  (error) => Promise.reject(toApiError(error))
);

apiClient.interceptors.response.use(
  (response) => response,
  (error) => Promise.reject(toApiError(error))
);

export const getApiClient = () => apiClient;

export const apiRequest = async <T>(
  config: AxiosRequestConfig
): Promise<T> => {
  try {
    const response = await apiClient.request<T>(config);
    return response.data;
  } catch (error) {
    throw toApiError(error);
  }
};

export const apiGet = async <T>(
  url: string,
  config: AxiosRequestConfig = {}
): Promise<T> => apiRequest<T>({ ...config, url, method: "get" });

export const apiPost = async <T>(
  url: string,
  data?: unknown,
  config: AxiosRequestConfig = {}
): Promise<T> => apiRequest<T>({ ...config, url, data, method: "post" });

// Masking API
export const getMaskingConfig = async (): Promise<MaskingConfig> => {
  return apiGet<MaskingConfig>("/api/v1/masking/config");
};

export const saveMaskingConfig = async (config: MaskingConfig): Promise<MaskingConfig> => {
  return apiPost<MaskingConfig>("/api/v1/masking/config", config);
};

export const testMasking = async (request: MaskingTestRequest): Promise<MaskingTestResponse> => {
  return apiPost<MaskingTestResponse>("/api/v1/masking/test", request);
};

// Backend Config API
export const getBackendConfig = async (): Promise<BackendConfig> => {
  return apiGet<BackendConfig>("/api/v1/config");
};

export const saveBackendConfig = async (config: BackendConfig): Promise<BackendConfig> => {
  return apiPost<BackendConfig>("/api/v1/config", config);
};

export const getBackendConfigSection = async <T>(section: string): Promise<T> => {
  return apiGet<T>(`/api/v1/config/${section}`);
};

export const updateBackendConfigSection = async <T>(section: string, data: T): Promise<T> => {
  return apiPost<T>(`/api/v1/config/${section}`, data);
};

// Prompts API
export const getPrompts = async (): Promise<PromptListResponse> => {
  return apiGet<PromptListResponse>("/api/v1/prompts");
};

export const createPrompt = async (prompt: Omit<SystemPrompt, "id">): Promise<PromptResponse> => {
  return apiPost<PromptResponse>("/api/v1/prompts", prompt);
};

export const updatePrompt = async (id: string, prompt: Partial<SystemPrompt>): Promise<PromptResponse> => {
  return apiPost<PromptResponse>(`/api/v1/prompts/${id}`, prompt);
};

export const deletePrompt = async (id: string): Promise<void> => {
  return apiPost<void>(`/api/v1/prompts/${id}/delete`);
};

export const setDefaultPrompt = async (id: string): Promise<PromptResponse> => {
  return apiPost<PromptResponse>(`/api/v1/prompts/${id}/default`);
};

// Memories API
export const getMemories = async (): Promise<MemoryListResponse> => {
  return apiGet<MemoryListResponse>("/api/v1/memories");
};

export const getSessionMemory = async (sessionId: string): Promise<SessionMemoryResponse> => {
  return apiGet<SessionMemoryResponse>(`/api/v1/sessions/${sessionId}/memory`);
};
