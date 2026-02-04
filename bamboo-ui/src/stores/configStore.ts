import { create } from "zustand";
import { persist } from "zustand/middleware";

import type { Config } from "@/types";

// 环境变量优先级最高
const envApiUrl = typeof process !== "undefined" 
  ? process.env.NEXT_PUBLIC_API_URL 
  : undefined;
const envWsUrl = typeof process !== "undefined"
  ? process.env.NEXT_PUBLIC_WS_URL
  : undefined;

// 默认值
const defaultApiUrl = "http://localhost:3000";
const defaultWsUrl = "ws://localhost:18790";

// 获取初始值（环境变量 > localStorage > 默认值）
const getInitialApiUrl = () => envApiUrl || defaultApiUrl;
const getInitialWsUrl = () => envWsUrl || defaultWsUrl;

const defaultConfig: Config = {
  apiUrl: getInitialApiUrl(),
  wsUrl: getInitialWsUrl(),
  model: "gpt-4o-mini",
  systemPrompt: "",
};

export interface ConfigState {
  config: Config;
  setApiUrl: (apiUrl: string) => void;
  setWsUrl: (wsUrl: string) => void;
  setModel: (model: string) => void;
  setSystemPrompt: (systemPrompt: string) => void;
  setApiKey: (apiKey?: string) => void;
  resetConfig: () => void;
  // 连接测试
  testConnection: () => Promise<{ success: boolean; latency?: number; error?: string }>;
}

export const useConfigStore = create<ConfigState>()(
  persist(
    (set, get) => ({
      config: defaultConfig,
      
      setApiUrl: (apiUrl) => {
        const next = apiUrl.trim();
        if (!next) {
          console.warn("API URL is empty.");
          return;
        }
        set((state) => ({
          config: { ...state.config, apiUrl: next },
        }));
      },
      
      setWsUrl: (wsUrl) => {
        const next = wsUrl.trim();
        if (!next) {
          console.warn("WebSocket URL is empty.");
          return;
        }
        set((state) => ({
          config: { ...state.config, wsUrl: next },
        }));
      },
      
      setModel: (model) => {
        const next = model.trim();
        if (!next) {
          console.warn("Model name is empty.");
          return;
        }
        set((state) => ({
          config: { ...state.config, model: next },
        }));
      },
      
      setSystemPrompt: (systemPrompt) => {
        set((state) => ({
          config: { ...state.config, systemPrompt },
        }));
      },
      
      setApiKey: (apiKey) => {
        set((state) => ({
          config: {
            ...state.config,
            apiKey: apiKey?.trim() || undefined,
          },
        }));
      },
      
      resetConfig: () => {
        set({ config: defaultConfig });
      },
      
      testConnection: async () => {
        const { apiUrl } = get().config;
        const startTime = Date.now();
        
        try {
          const response = await fetch(`${apiUrl}/health`, {
            method: "GET",
            headers: { "Content-Type": "application/json" },
          });
          
          const latency = Date.now() - startTime;
          
          if (response.ok) {
            return { success: true, latency };
          } else {
            return { 
              success: false, 
              latency,
              error: `HTTP ${response.status}: ${response.statusText}` 
            };
          }
        } catch (error) {
          return { 
            success: false, 
            error: error instanceof Error ? error.message : "Connection failed" 
          };
        }
      },
    }),
    {
      name: "bamboo-config",
      partialize: (state) => ({ config: state.config }),
    }
  )
);
