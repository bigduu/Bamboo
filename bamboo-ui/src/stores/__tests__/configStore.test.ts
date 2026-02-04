import { describe, it, expect, beforeEach, vi } from "vitest";
import type { Config } from "@/types";

// Simple state container for testing
class TestConfigStore {
  config: Config = {
    apiUrl: "http://localhost:3000",
    wsUrl: "ws://localhost:18790",
    model: "gpt-4o-mini",
    systemPrompt: "",
  };

  setApiUrl(apiUrl: string) {
    const next = apiUrl.trim();
    if (!next) {
      console.warn("API URL is empty.");
      return;
    }
    this.config = { ...this.config, apiUrl: next };
  }

  setWsUrl(wsUrl: string) {
    const next = wsUrl.trim();
    if (!next) {
      console.warn("WebSocket URL is empty.");
      return;
    }
    this.config = { ...this.config, wsUrl: next };
  }

  setModel(model: string) {
    const next = model.trim();
    if (!next) {
      console.warn("Model name is empty.");
      return;
    }
    this.config = { ...this.config, model: next };
  }

  setSystemPrompt(systemPrompt: string) {
    this.config = { ...this.config, systemPrompt };
  }

  setApiKey(apiKey?: string) {
    this.config = {
      ...this.config,
      apiKey: apiKey?.trim() || undefined,
    };
  }

  resetConfig() {
    this.config = {
      apiUrl: "http://localhost:3000",
      wsUrl: "ws://localhost:18790",
      model: "gpt-4o-mini",
      systemPrompt: "",
    };
  }

  async testConnection(): Promise<{ success: boolean; latency?: number; error?: string }> {
    const { apiUrl } = this.config;
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
  }
}

describe("configStore", () => {
  let store: TestConfigStore;

  beforeEach(() => {
    store = new TestConfigStore();
  });

  describe("initial state", () => {
    it("should have default config values", () => {
      expect(store.config.apiUrl).toBe("http://localhost:3000");
      expect(store.config.wsUrl).toBe("ws://localhost:18790");
      expect(store.config.model).toBe("gpt-4o-mini");
      expect(store.config.systemPrompt).toBe("");
    });
  });

  describe("setApiUrl", () => {
    it("should update API URL", () => {
      store.setApiUrl("http://new-api.com:8080");
      
      expect(store.config.apiUrl).toBe("http://new-api.com:8080");
    });

    it("should trim whitespace from URL", () => {
      store.setApiUrl("  http://new-api.com  ");
      
      expect(store.config.apiUrl).toBe("http://new-api.com");
    });

    it("should not update with empty URL", () => {
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      
      store.setApiUrl("http://valid-url.com");
      store.setApiUrl("");
      
      expect(store.config.apiUrl).toBe("http://valid-url.com");
      consoleSpy.mockRestore();
    });

    it("should not update with whitespace-only URL", () => {
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      
      store.setApiUrl("http://valid-url.com");
      store.setApiUrl("   ");
      
      expect(store.config.apiUrl).toBe("http://valid-url.com");
      consoleSpy.mockRestore();
    });
  });

  describe("setWsUrl", () => {
    it("should update WebSocket URL", () => {
      store.setWsUrl("ws://new-ws.com:9000");
      
      expect(store.config.wsUrl).toBe("ws://new-ws.com:9000");
    });

    it("should trim whitespace from URL", () => {
      store.setWsUrl("  ws://new-ws.com  ");
      
      expect(store.config.wsUrl).toBe("ws://new-ws.com");
    });

    it("should not update with empty URL", () => {
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      
      store.setWsUrl("ws://valid-ws.com");
      store.setWsUrl("");
      
      expect(store.config.wsUrl).toBe("ws://valid-ws.com");
      consoleSpy.mockRestore();
    });
  });

  describe("setModel", () => {
    it("should update model", () => {
      store.setModel("gpt-4");
      
      expect(store.config.model).toBe("gpt-4");
    });

    it("should trim whitespace from model", () => {
      store.setModel("  gpt-4-turbo  ");
      
      expect(store.config.model).toBe("gpt-4-turbo");
    });

    it("should not update with empty model", () => {
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      
      store.setModel("gpt-4");
      store.setModel("");
      
      expect(store.config.model).toBe("gpt-4");
      consoleSpy.mockRestore();
    });
  });

  describe("setSystemPrompt", () => {
    it("should update system prompt", () => {
      store.setSystemPrompt("You are a helpful assistant.");
      
      expect(store.config.systemPrompt).toBe("You are a helpful assistant.");
    });

    it("should allow empty system prompt", () => {
      store.setSystemPrompt("Some prompt");
      
      store.setSystemPrompt("");
      
      expect(store.config.systemPrompt).toBe("");
    });
  });

  describe("setApiKey", () => {
    it("should update API key", () => {
      store.setApiKey("sk-test123");
      
      expect(store.config.apiKey).toBe("sk-test123");
    });

    it("should trim whitespace from API key", () => {
      store.setApiKey("  sk-test123  ");
      
      expect(store.config.apiKey).toBe("sk-test123");
    });

    it("should remove API key when set to undefined", () => {
      store.setApiKey("sk-test123");
      
      store.setApiKey(undefined);
      
      expect(store.config.apiKey).toBeUndefined();
    });

    it("should remove API key when set to empty string", () => {
      store.setApiKey("sk-test123");
      
      store.setApiKey("");
      
      expect(store.config.apiKey).toBeUndefined();
    });
  });

  describe("resetConfig", () => {
    it("should reset all config to defaults", () => {
      store.setApiUrl("http://custom.com");
      store.setWsUrl("ws://custom.com");
      store.setModel("custom-model");
      store.setSystemPrompt("Custom prompt");
      store.setApiKey("custom-key");
      
      store.resetConfig();
      
      expect(store.config.apiUrl).toBe("http://localhost:3000");
      expect(store.config.wsUrl).toBe("ws://localhost:18790");
      expect(store.config.model).toBe("gpt-4o-mini");
      expect(store.config.systemPrompt).toBe("");
      expect(store.config.apiKey).toBeUndefined();
    });
  });

  describe("testConnection", () => {
    it("should return success on successful connection", async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        statusText: "OK",
      });
      
      const result = await store.testConnection();
      
      expect(result.success).toBe(true);
      expect(result.latency).toBeDefined();
      expect(result.error).toBeUndefined();
    });

    it("should return error on failed connection", async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        statusText: "Internal Server Error",
      });
      
      const result = await store.testConnection();
      
      expect(result.success).toBe(false);
      expect(result.error).toContain("500");
    });

    it("should return error on network failure", async () => {
      global.fetch = vi.fn().mockRejectedValue(new Error("Network error"));
      
      const result = await store.testConnection();
      
      expect(result.success).toBe(false);
      expect(result.error).toBe("Network error");
    });

    it("should return error on unknown error", async () => {
      global.fetch = vi.fn().mockRejectedValue("Unknown error");
      
      const result = await store.testConnection();
      
      expect(result.success).toBe(false);
      expect(result.error).toBe("Connection failed");
    });
  });
});
