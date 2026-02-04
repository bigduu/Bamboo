export type Id = string;

export type MessageRole = "system" | "user" | "assistant" | "tool";

export interface ToolCall {
  id: Id;
  name: string;
  args: Record<string, unknown>;
  result?: unknown;
  error?: string;
}

export interface Message {
  id: Id;
  role: MessageRole;
  content: string;
  createdAt: string;
  toolCalls?: ToolCall[];
  toolCallId?: Id;
  status?: "pending" | "streaming" | "completed" | "error";
  error?: string;
}

export interface Session {
  id: Id;
  title: string;
  createdAt: string;
  updatedAt: string;
}

export interface Config {
  apiUrl: string;
  wsUrl: string;
  model: string;
  systemPrompt: string;
  apiKey?: string;
}

// Backend Config Types
export interface ServerConfig {
  port: number;
  host: string;
  cors: boolean;
}

export interface GatewayConfig {
  enabled: boolean;
  bind: string;
  auth_token: string | null;
  max_connections: number;
  heartbeat_interval_secs: number;
}

export interface LlmProviderConfig {
  enabled: boolean;
  base_url: string;
  model: string;
  auth_type: string;
  env: string;
  timeout_seconds: number;
}

export interface LlmConfig {
  default_provider: string;
  providers: Record<string, LlmProviderConfig>;
}

export interface SkillsConfig {
  enabled: boolean;
  auto_reload: boolean;
  directories: string[];
}

export interface AgentConfig {
  max_rounds: number;
  system_prompt: string;
  timeout_seconds: number;
}

export interface StorageConfig {
  type: string;
  path: string;
}

export interface LoggingConfig {
  level: string;
  file: string;
  max_size_mb: number;
  max_files: number;
}

export interface BackendConfig {
  version: string;
  server: ServerConfig;
  gateway: GatewayConfig;
  llm: LlmConfig;
  skills: SkillsConfig;
  agent: AgentConfig;
  storage: StorageConfig;
  logging: LoggingConfig;
}

export interface ApiError {
  message: string;
  status?: number;
  code?: string;
  details?: unknown;
}

export interface ChatRequest {
  sessionId: Id;
  messages: Message[];
  model: string;
  systemPrompt?: string;
  stream?: boolean;
}

export interface ChatResponse {
  message: Message;
  usage?: Record<string, number>;
}

export interface StreamDelta {
  type: "delta";
  content: string;
}

export interface StreamDone {
  type: "done";
}

export interface StreamError {
  type: "error";
  message: string;
  details?: unknown;
}

export type StreamEvent = StreamDelta | StreamDone | StreamError;

export type WebSocketStatus = "idle" | "connecting" | "open" | "closed" | "error";

// Masking Types
export interface MaskingRule {
  id: string;
  name: string;
  pattern: string;
  replacement: string;
  enabled: boolean;
  description?: string;
  isRegex: boolean;
}

export interface MaskingConfig {
  enabled: boolean;
  rules: MaskingRule[];
}

export interface MaskingTestRequest {
  text: string;
  config: MaskingConfig;
}

export interface MaskingTestResponse {
  original: string;
  masked: string;
  matches: Array<{
    ruleId: string;
    ruleName: string;
    matched: string;
    position: { start: number; end: number };
  }>;
}

// Prompt Types
export interface SystemPrompt {
  id: string;
  name: string;
  content: string;
  is_default: boolean;
  is_custom: boolean;
  category: string;
}

export interface PromptListResponse {
  prompts: SystemPrompt[];
}

export interface PromptResponse {
  prompt: SystemPrompt;
}

// Memory Types
export interface Memory {
  id: string;
  session_id: string;
  content: string;
  tags: string[];
  created_at: string;
  updated_at?: string;
}

export interface SessionMemory {
  session_id: string;
  memories: Memory[];
  updated_at: string;
}

export interface MemoryListResponse {
  memories: Memory[];
}

export interface SessionMemoryResponse {
  session_memory: SessionMemory;
}
