import type { Config } from "@/types";

export const mockConfig: Config = {
  apiUrl: "http://localhost:3000",
  wsUrl: "ws://localhost:18790",
  model: "gpt-4o-mini",
  systemPrompt: "",
};

export const createMockMessage = (overrides = {}) => ({
  id: "msg_123",
  role: "user" as const,
  content: "Hello",
  createdAt: new Date().toISOString(),
  ...overrides,
});

export const createMockSession = (overrides = {}) => ({
  id: "session_123",
  title: "Test Session",
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
  ...overrides,
});
