import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { useState } from "react";
import type { Config } from "@/types";

// Mock settings page component
const MockSettingsPage = () => {
  const [config, setConfig] = useState<Config>({
    apiUrl: "http://localhost:3000",
    wsUrl: "ws://localhost:18790",
    model: "gpt-4o-mini",
    systemPrompt: "",
  });
  const [testResult, setTestResult] = useState<{ success?: boolean; error?: string } | null>(null);
  
  const handleTest = async () => {
    try {
      const response = await fetch(`${config.apiUrl}/health`);
      if (response.ok) {
        setTestResult({ success: true });
      } else {
        setTestResult({ success: false, error: "Connection failed" });
      }
    } catch {
      setTestResult({ success: false, error: "Network error" });
    }
  };
  
  const resetConfig = () => {
    setConfig({
      apiUrl: "http://localhost:3000",
      wsUrl: "ws://localhost:18790",
      model: "gpt-4o-mini",
      systemPrompt: "",
    });
  };
  
  return (
    <div data-testid="settings-page">
      <section data-testid="server-config">
        <h2>Server Configuration</h2>
        <label>API URL</label>
        <input
          data-testid="api-url-input"
          value={config.apiUrl}
          onChange={(e) => setConfig({ ...config, apiUrl: e.target.value })}
        />
        
        <label>WebSocket URL</label>
        <input
          data-testid="ws-url-input"
          value={config.wsUrl}
          onChange={(e) => setConfig({ ...config, wsUrl: e.target.value })}
        />
        
        <button data-testid="test-connection" onClick={handleTest}>
          Test Connection
        </button>
        
        {testResult && (
          <div data-testid="test-result">
            {testResult.success ? "Connected" : testResult.error}
          </div>
        )}
      </section>
      
      <section data-testid="model-config">
        <h2>Model Configuration</h2>
        <label>Model</label>
        <input
          data-testid="model-input"
          value={config.model}
          onChange={(e) => setConfig({ ...config, model: e.target.value })}
        />
        
        <label>System Prompt</label>
        <textarea
          data-testid="system-prompt-input"
          value={config.systemPrompt}
          onChange={(e) => setConfig({ ...config, systemPrompt: e.target.value })}
        />
      </section>
      
      <button data-testid="reset-config" onClick={resetConfig}>
        Reset to Defaults
      </button>
    </div>
  );
};

describe("Settings Flow Integration", () => {
  beforeEach(() => {
    // Mock fetch for connection test
    global.fetch = vi.fn();
  });

  it("should display current configuration", () => {
    render(<MockSettingsPage />);
    
    expect(screen.getByTestId("api-url-input")).toHaveValue("http://localhost:3000");
    expect(screen.getByTestId("ws-url-input")).toHaveValue("ws://localhost:18790");
    expect(screen.getByTestId("model-input")).toHaveValue("gpt-4o-mini");
  });

  it("should update API URL", () => {
    render(<MockSettingsPage />);
    
    const input = screen.getByTestId("api-url-input");
    fireEvent.change(input, { target: { value: "http://new-api.com:8080" } });
    
    expect(input).toHaveValue("http://new-api.com:8080");
  });

  it("should update WebSocket URL", () => {
    render(<MockSettingsPage />);
    
    const input = screen.getByTestId("ws-url-input");
    fireEvent.change(input, { target: { value: "ws://new-ws.com:9000" } });
    
    expect(input).toHaveValue("ws://new-ws.com:9000");
  });

  it("should update model", () => {
    render(<MockSettingsPage />);
    
    const input = screen.getByTestId("model-input");
    fireEvent.change(input, { target: { value: "gpt-4" } });
    
    expect(input).toHaveValue("gpt-4");
  });

  it("should update system prompt", () => {
    render(<MockSettingsPage />);
    
    const input = screen.getByTestId("system-prompt-input");
    fireEvent.change(input, { target: { value: "You are a helpful assistant." } });
    
    expect(input).toHaveValue("You are a helpful assistant.");
  });

  it("should test connection successfully", async () => {
    vi.mocked(global.fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
    } as Response);
    
    render(<MockSettingsPage />);
    
    fireEvent.click(screen.getByTestId("test-connection"));
    
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        "http://localhost:3000/health"
      );
    });
  });

  it("should handle connection test failure", async () => {
    vi.mocked(global.fetch).mockResolvedValueOnce({
      ok: false,
      status: 500,
    } as Response);
    
    render(<MockSettingsPage />);
    
    fireEvent.click(screen.getByTestId("test-connection"));
    
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalled();
    });
  });

  it("should reset configuration to defaults", () => {
    render(<MockSettingsPage />);
    
    // Change some values
    fireEvent.change(screen.getByTestId("api-url-input"), {
      target: { value: "http://custom.com" },
    });
    fireEvent.change(screen.getByTestId("model-input"), {
      target: { value: "custom-model" },
    });
    
    // Reset
    fireEvent.click(screen.getByTestId("reset-config"));
    
    // Should be back to defaults
    expect(screen.getByTestId("api-url-input")).toHaveValue("http://localhost:3000");
    expect(screen.getByTestId("model-input")).toHaveValue("gpt-4o-mini");
  });
});
