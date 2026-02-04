"use client";

import { useCallback, useRef } from "react";

import { apiPost } from "@/lib/api";
import { useChatStore } from "@/stores/chatStore";
import { useConfigStore } from "@/stores/configStore";
import { useSessionStore } from "@/stores/sessionStore";
import type { ChatResponse, Id, Message } from "@/types";

const createId = (prefix: string) => {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `${prefix}_${crypto.randomUUID()}`;
  }

  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
};

const nowIso = () => new Date().toISOString();

const resolveEndpoint = (apiUrl: string, endpoint: string) => {
  if (!apiUrl) {
    return endpoint;
  }

  const normalizedApiUrl = apiUrl.replace(/\/$/, "");
  const normalizedEndpoint = endpoint.startsWith("/")
    ? endpoint
    : `/${endpoint}`;

  return `${normalizedApiUrl}${normalizedEndpoint}`;
};

const extractDelta = (data: string): string => {
  try {
    const parsed = JSON.parse(data) as Record<string, unknown>;

    if (parsed?.type === "error" && typeof parsed.message === "string") {
      throw new Error(parsed.message);
    }

    const direct = parsed?.delta ?? parsed?.content;
    if (typeof direct === "string") {
      return direct;
    }

    const choice = (parsed as { choices?: Array<Record<string, unknown>> }).choices?.[0];
    if (choice) {
      const delta = (choice.delta as { content?: unknown } | undefined)?.content;
      if (typeof delta === "string") {
        return delta;
      }

      const text = choice.text;
      if (typeof text === "string") {
        return text;
      }
    }

    const message = (parsed as { message?: { content?: unknown } }).message?.content;
    if (typeof message === "string") {
      return message;
    }
  } catch (error) {
    if (error instanceof SyntaxError) {
      return data;
    }

    if (error instanceof Error && error.message) {
      throw error;
    }
  }

  return data;
};

export interface SendMessageOptions {
  stream?: boolean;
  endpoint?: string;
  headers?: Record<string, string>;
}

export interface UseChatResult {
  sessionId: Id | null;
  messages: Message[];
  isSending: boolean;
  streamingMessageId: Id | null;
  sendMessage: (content: string, options?: SendMessageOptions) => Promise<void>;
  cancelStreaming: () => void;
}

export const useChat = (sessionId?: Id): UseChatResult => {
  const currentSessionId = useSessionStore((state) => state.currentSessionId);
  const activeSessionId = sessionId ?? currentSessionId;
  const { apiUrl, model, systemPrompt, apiKey } = useConfigStore(
    (state) => state.config
  );

  const messages = useChatStore((state) =>
    activeSessionId ? state.messagesBySession[activeSessionId] ?? [] : []
  );
  const isSending = useChatStore((state) =>
    activeSessionId ? state.sendingBySession[activeSessionId] ?? false : false
  );
  const streamingMessageId = useChatStore((state) =>
    activeSessionId
      ? state.streamingMessageIdBySession[activeSessionId] ?? null
      : null
  );

  const addMessage = useChatStore((state) => state.addMessage);
  const setSending = useChatStore((state) => state.setSending);
  const startStreamingMessage = useChatStore(
    (state) => state.startStreamingMessage
  );
  const appendStreamingMessage = useChatStore(
    (state) => state.appendStreamingMessage
  );
  const finishStreamingMessage = useChatStore(
    (state) => state.finishStreamingMessage
  );
  const failStreamingMessage = useChatStore(
    (state) => state.failStreamingMessage
  );

  const abortRef = useRef<AbortController | null>(null);

  const cancelStreaming = useCallback(() => {
    if (abortRef.current) {
      abortRef.current.abort();
      abortRef.current = null;
    }
  }, []);

  const sendMessage = useCallback(
    async (content: string, options?: SendMessageOptions) => {
      if (!activeSessionId) {
        throw new Error("No active session.");
      }

      const trimmed = content.trim();
      if (!trimmed) {
        throw new Error("Message content is empty.");
      }

      const userMessage: Message = {
        id: createId("message"),
        role: "user",
        content: trimmed,
        createdAt: nowIso(),
        status: "completed",
      };

      addMessage(activeSessionId, userMessage);
      setSending(activeSessionId, true);

      const payload = {
        sessionId: activeSessionId,
        messages: [...messages, userMessage],
        model,
        systemPrompt,
        stream: options?.stream ?? true,
      };

      try {
        if (options?.stream ?? true) {
          cancelStreaming();
          const controller = new AbortController();
          abortRef.current = controller;

          const endpoint = options?.endpoint ?? "/chat/stream";
          const url = resolveEndpoint(apiUrl, endpoint);

          const response = await fetch(url, {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              Accept: "text/event-stream",
              ...(apiKey ? { Authorization: `Bearer ${apiKey}` } : {}),
              ...(options?.headers ?? {}),
            },
            body: JSON.stringify(payload),
            signal: controller.signal,
          });

          if (!response.ok) {
            throw new Error(
              `SSE request failed with status ${response.status}.`
            );
          }

          if (!response.body) {
            throw new Error("SSE response has no body.");
          }

          startStreamingMessage(activeSessionId, "");

          const reader = response.body.getReader();
          const decoder = new TextDecoder("utf-8");
          let buffer = "";

          while (true) {
            const { value, done } = await reader.read();
            if (done) {
              break;
            }

            buffer += decoder.decode(value, { stream: true });
            const lines = buffer.split("\n");
            buffer = lines.pop() ?? "";

            for (const line of lines) {
              const trimmedLine = line.trim();
              if (!trimmedLine.startsWith("data:")) {
                continue;
              }

              const data = trimmedLine.replace(/^data:\s?/, "");
              if (!data) {
                continue;
              }

              if (data === "[DONE]") {
                finishStreamingMessage(activeSessionId);
                return;
              }

              try {
                const delta = extractDelta(data);
                if (delta) {
                  appendStreamingMessage(activeSessionId, delta);
                }
              } catch (error) {
                const message =
                  error instanceof Error ? error.message : "Stream error";
                failStreamingMessage(activeSessionId, message);
                throw error;
              }
            }
          }

          finishStreamingMessage(activeSessionId);
        } else {
          const endpoint = options?.endpoint ?? "/chat";
          const response = await apiPost<ChatResponse>(endpoint, payload, {
            headers: options?.headers,
          });

          const assistantMessage: Message = {
            id: response.message.id || createId("message"),
            role: response.message.role ?? "assistant",
            content: response.message.content ?? "",
            createdAt: response.message.createdAt ?? nowIso(),
            status: "completed",
          };

          addMessage(activeSessionId, assistantMessage);
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : "Unknown error";
        failStreamingMessage(activeSessionId, message);
        throw error;
      } finally {
        setSending(activeSessionId, false);
        abortRef.current = null;
      }
    },
    [
      activeSessionId,
      addMessage,
      apiKey,
      apiUrl,
      appendStreamingMessage,
      cancelStreaming,
      failStreamingMessage,
      finishStreamingMessage,
      messages,
      model,
      setSending,
      startStreamingMessage,
      systemPrompt,
    ]
  );

  return {
    sessionId: activeSessionId ?? null,
    messages,
    isSending,
    streamingMessageId,
    sendMessage,
    cancelStreaming,
  };
};
