import { create } from "zustand";

import type { Id, Message } from "@/types";

const createId = (prefix: string) => {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `${prefix}_${crypto.randomUUID()}`;
  }

  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
};

const nowIso = () => new Date().toISOString();

export interface ChatState {
  messagesBySession: Record<Id, Message[]>;
  sendingBySession: Record<Id, boolean>;
  streamingMessageIdBySession: Record<Id, Id | null>;
  addMessage: (sessionId: Id, message: Message) => void;
  setMessages: (sessionId: Id, messages: Message[]) => void;
  updateMessage: (sessionId: Id, messageId: Id, updates: Partial<Message>) => void;
  setSending: (sessionId: Id, isSending: boolean) => void;
  startStreamingMessage: (sessionId: Id, initialContent?: string) => Message;
  appendStreamingMessage: (sessionId: Id, delta: string) => void;
  finishStreamingMessage: (sessionId: Id) => void;
  failStreamingMessage: (sessionId: Id, error: string) => void;
  clearSession: (sessionId: Id) => void;
}

export const useChatStore = create<ChatState>((set, get) => ({
  messagesBySession: {},
  sendingBySession: {},
  streamingMessageIdBySession: {},
  addMessage: (sessionId, message) => {
    if (!sessionId) {
      console.error("Missing sessionId when adding a message.");
      return;
    }

    set((state) => ({
      messagesBySession: {
        ...state.messagesBySession,
        [sessionId]: [...(state.messagesBySession[sessionId] ?? []), message],
      },
    }));
  },
  setMessages: (sessionId, messages) => {
    if (!sessionId) {
      console.error("Missing sessionId when setting messages.");
      return;
    }

    set((state) => ({
      messagesBySession: {
        ...state.messagesBySession,
        [sessionId]: messages,
      },
    }));
  },
  updateMessage: (sessionId, messageId, updates) => {
    if (!sessionId) {
      console.error("Missing sessionId when updating a message.");
      return;
    }

    set((state) => {
      const messages = state.messagesBySession[sessionId] ?? [];
      const index = messages.findIndex((message) => message.id === messageId);
      if (index === -1) {
        console.warn(`Message not found: ${messageId}`);
        return state;
      }

      const nextMessages = [...messages];
      nextMessages[index] = { ...nextMessages[index], ...updates };

      return {
        messagesBySession: {
          ...state.messagesBySession,
          [sessionId]: nextMessages,
        },
      };
    });
  },
  setSending: (sessionId, isSending) => {
    if (!sessionId) {
      console.error("Missing sessionId when setting sending state.");
      return;
    }

    set((state) => ({
      sendingBySession: {
        ...state.sendingBySession,
        [sessionId]: isSending,
      },
    }));
  },
  startStreamingMessage: (sessionId, initialContent = "") => {
    if (!sessionId) {
      throw new Error("Missing sessionId when starting streaming message.");
    }

    const message: Message = {
      id: createId("message"),
      role: "assistant",
      content: initialContent,
      createdAt: nowIso(),
      status: "streaming",
    };

    set((state) => ({
      messagesBySession: {
        ...state.messagesBySession,
        [sessionId]: [...(state.messagesBySession[sessionId] ?? []), message],
      },
      streamingMessageIdBySession: {
        ...state.streamingMessageIdBySession,
        [sessionId]: message.id,
      },
    }));

    return message;
  },
  appendStreamingMessage: (sessionId, delta) => {
    if (!sessionId) {
      console.error("Missing sessionId when appending streaming message.");
      return;
    }

    if (!delta) {
      return;
    }

    set((state) => {
      const messageId = state.streamingMessageIdBySession[sessionId];
      if (!messageId) {
        console.warn(`No streaming message to append for session: ${sessionId}`);
        return state;
      }

      const messages = state.messagesBySession[sessionId] ?? [];
      const index = messages.findIndex((message) => message.id === messageId);
      if (index === -1) {
        console.warn(`Streaming message not found: ${messageId}`);
        return state;
      }

      const nextMessages = [...messages];
      const current = nextMessages[index];
      nextMessages[index] = {
        ...current,
        content: `${current.content}${delta}`,
        status: "streaming",
      };

      return {
        messagesBySession: {
          ...state.messagesBySession,
          [sessionId]: nextMessages,
        },
      };
    });
  },
  finishStreamingMessage: (sessionId) => {
    if (!sessionId) {
      console.error("Missing sessionId when finishing streaming message.");
      return;
    }

    set((state) => {
      const messageId = state.streamingMessageIdBySession[sessionId];
      if (!messageId) {
        return {
          streamingMessageIdBySession: {
            ...state.streamingMessageIdBySession,
            [sessionId]: null,
          },
        };
      }

      const messages = state.messagesBySession[sessionId] ?? [];
      const index = messages.findIndex((message) => message.id === messageId);
      if (index === -1) {
        return {
          streamingMessageIdBySession: {
            ...state.streamingMessageIdBySession,
            [sessionId]: null,
          },
        };
      }

      const nextMessages = [...messages];
      nextMessages[index] = {
        ...nextMessages[index],
        status: "completed",
      };

      return {
        messagesBySession: {
          ...state.messagesBySession,
          [sessionId]: nextMessages,
        },
        streamingMessageIdBySession: {
          ...state.streamingMessageIdBySession,
          [sessionId]: null,
        },
      };
    });
  },
  failStreamingMessage: (sessionId, error) => {
    if (!sessionId) {
      console.error("Missing sessionId when failing streaming message.");
      return;
    }

    set((state) => {
      const messageId = state.streamingMessageIdBySession[sessionId];
      if (!messageId) {
        return state;
      }

      const messages = state.messagesBySession[sessionId] ?? [];
      const index = messages.findIndex((message) => message.id === messageId);
      if (index === -1) {
        return state;
      }

      const nextMessages = [...messages];
      nextMessages[index] = {
        ...nextMessages[index],
        status: "error",
        error,
      };

      return {
        messagesBySession: {
          ...state.messagesBySession,
          [sessionId]: nextMessages,
        },
        streamingMessageIdBySession: {
          ...state.streamingMessageIdBySession,
          [sessionId]: null,
        },
      };
    });
  },
  clearSession: (sessionId) => {
    if (!sessionId) {
      console.error("Missing sessionId when clearing messages.");
      return;
    }

    set((state) => ({
      messagesBySession: {
        ...state.messagesBySession,
        [sessionId]: [],
      },
      sendingBySession: {
        ...state.sendingBySession,
        [sessionId]: false,
      },
      streamingMessageIdBySession: {
        ...state.streamingMessageIdBySession,
        [sessionId]: null,
      },
    }));
  },
}));
