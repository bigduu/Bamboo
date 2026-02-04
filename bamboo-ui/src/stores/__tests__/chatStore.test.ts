import { describe, it, expect, beforeEach, vi } from "vitest";

// Simple state container for testing
class TestChatStore {
  messagesBySession: Record<string, Array<{ id: string; role: string; content: string; createdAt: string; status?: string; error?: string }>> = {};
  sendingBySession: Record<string, boolean> = {};
  streamingMessageIdBySession: Record<string, string | null> = {};

  createId(prefix: string) {
    return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
  }

  nowIso() {
    return new Date().toISOString();
  }

  addMessage(sessionId: string, message: { id: string; role: string; content: string; createdAt: string }) {
    if (!sessionId) {
      console.error("Missing sessionId when adding a message.");
      return;
    }

    this.messagesBySession = {
      ...this.messagesBySession,
      [sessionId]: [...(this.messagesBySession[sessionId] ?? []), message],
    };
  }

  setMessages(sessionId: string, messages: typeof this.messagesBySession[string]) {
    if (!sessionId) {
      console.error("Missing sessionId when setting messages.");
      return;
    }

    this.messagesBySession = {
      ...this.messagesBySession,
      [sessionId]: messages,
    };
  }

  updateMessage(sessionId: string, messageId: string, updates: Partial<{ content: string; status: string }>) {
    if (!sessionId) {
      console.error("Missing sessionId when updating a message.");
      return;
    }

    const messages = this.messagesBySession[sessionId] ?? [];
    const index = messages.findIndex((message) => message.id === messageId);
    if (index === -1) {
      console.warn(`Message not found: ${messageId}`);
      return;
    }

    const nextMessages = [...messages];
    nextMessages[index] = { ...nextMessages[index], ...updates };

    this.messagesBySession = {
      ...this.messagesBySession,
      [sessionId]: nextMessages,
    };
  }

  setSending(sessionId: string, isSending: boolean) {
    if (!sessionId) {
      console.error("Missing sessionId when setting sending state.");
      return;
    }

    this.sendingBySession = {
      ...this.sendingBySession,
      [sessionId]: isSending,
    };
  }

  startStreamingMessage(sessionId: string, initialContent = "") {
    if (!sessionId) {
      throw new Error("Missing sessionId when starting streaming message.");
    }

    const message = {
      id: this.createId("message"),
      role: "assistant",
      content: initialContent,
      createdAt: this.nowIso(),
      status: "streaming",
    };

    this.messagesBySession = {
      ...this.messagesBySession,
      [sessionId]: [...(this.messagesBySession[sessionId] ?? []), message],
    };
    this.streamingMessageIdBySession = {
      ...this.streamingMessageIdBySession,
      [sessionId]: message.id,
    };

    return message;
  }

  appendStreamingMessage(sessionId: string, delta: string) {
    if (!sessionId) {
      console.error("Missing sessionId when appending streaming message.");
      return;
    }

    if (!delta) {
      return;
    }

    const messageId = this.streamingMessageIdBySession[sessionId];
    if (!messageId) {
      console.warn(`No streaming message to append for session: ${sessionId}`);
      return;
    }

    const messages = this.messagesBySession[sessionId] ?? [];
    const index = messages.findIndex((message) => message.id === messageId);
    if (index === -1) {
      console.warn(`Streaming message not found: ${messageId}`);
      return;
    }

    const nextMessages = [...messages];
    const current = nextMessages[index];
    nextMessages[index] = {
      ...current,
      content: `${current.content}${delta}`,
      status: "streaming",
    };

    this.messagesBySession = {
      ...this.messagesBySession,
      [sessionId]: nextMessages,
    };
  }

  finishStreamingMessage(sessionId: string) {
    if (!sessionId) {
      console.error("Missing sessionId when finishing streaming message.");
      return;
    }

    const messageId = this.streamingMessageIdBySession[sessionId];
    if (!messageId) {
      this.streamingMessageIdBySession = {
        ...this.streamingMessageIdBySession,
        [sessionId]: null,
      };
      return;
    }

    const messages = this.messagesBySession[sessionId] ?? [];
    const index = messages.findIndex((message) => message.id === messageId);
    if (index === -1) {
      this.streamingMessageIdBySession = {
        ...this.streamingMessageIdBySession,
        [sessionId]: null,
      };
      return;
    }

    const nextMessages = [...messages];
    nextMessages[index] = {
      ...nextMessages[index],
      status: "completed",
    };

    this.messagesBySession = {
      ...this.messagesBySession,
      [sessionId]: nextMessages,
    };
    this.streamingMessageIdBySession = {
      ...this.streamingMessageIdBySession,
      [sessionId]: null,
    };
  }

  failStreamingMessage(sessionId: string, error: string) {
    if (!sessionId) {
      console.error("Missing sessionId when failing streaming message.");
      return;
    }

    const messageId = this.streamingMessageIdBySession[sessionId];
    if (!messageId) {
      return;
    }

    const messages = this.messagesBySession[sessionId] ?? [];
    const index = messages.findIndex((message) => message.id === messageId);
    if (index === -1) {
      return;
    }

    const nextMessages = [...messages];
    nextMessages[index] = {
      ...nextMessages[index],
      status: "error",
      error,
    };

    this.messagesBySession = {
      ...this.messagesBySession,
      [sessionId]: nextMessages,
    };
    this.streamingMessageIdBySession = {
      ...this.streamingMessageIdBySession,
      [sessionId]: null,
    };
  }

  clearSession(sessionId: string) {
    if (!sessionId) {
      console.error("Missing sessionId when clearing messages.");
      return;
    }

    this.messagesBySession = {
      ...this.messagesBySession,
      [sessionId]: [],
    };
    this.sendingBySession = {
      ...this.sendingBySession,
      [sessionId]: false,
    };
    this.streamingMessageIdBySession = {
      ...this.streamingMessageIdBySession,
      [sessionId]: null,
    };
  }
}

describe("chatStore", () => {
  let store: TestChatStore;
  const sessionId = "test-session-123";

  beforeEach(() => {
    store = new TestChatStore();
  });

  describe("addMessage", () => {
    it("should add a message to the session", () => {
      const message = {
        id: "msg-1",
        role: "user",
        content: "Hello",
        createdAt: "2024-01-01T00:00:00Z",
      };
      
      store.addMessage(sessionId, message);
      
      expect(store.messagesBySession[sessionId]).toHaveLength(1);
      expect(store.messagesBySession[sessionId][0]).toEqual(message);
    });

    it("should not add message without sessionId", () => {
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
      const message = {
        id: "msg-1",
        role: "user",
        content: "Hello",
        createdAt: "2024-01-01T00:00:00Z",
      };
      
      store.addMessage("", message);
      
      expect(Object.keys(store.messagesBySession)).not.toContain("");
      consoleSpy.mockRestore();
    });
  });

  describe("setMessages", () => {
    it("should set all messages for a session", () => {
      const messages = [
        { id: "msg-1", role: "user", content: "Hello", createdAt: "2024-01-01T00:00:00Z" },
        { id: "msg-2", role: "assistant", content: "Hi there", createdAt: "2024-01-01T00:00:01Z" },
      ];
      
      store.setMessages(sessionId, messages);
      
      expect(store.messagesBySession[sessionId]).toEqual(messages);
    });

    it("should create new array for session if not exists", () => {
      const newSessionId = "new-session";
      const messages = [{ id: "msg-1", role: "user", content: "Hello", createdAt: "2024-01-01T00:00:00Z" }];
      
      store.setMessages(newSessionId, messages);
      
      expect(store.messagesBySession[newSessionId]).toEqual(messages);
    });
  });

  describe("updateMessage", () => {
    it("should update message content", () => {
      const message = { id: "msg-1", role: "user", content: "Hello", createdAt: "2024-01-01T00:00:00Z" };
      store.addMessage(sessionId, message);
      
      store.updateMessage(sessionId, "msg-1", { content: "Updated content" });
      
      expect(store.messagesBySession[sessionId][0].content).toBe("Updated content");
    });

    it("should return unchanged state for non-existent message", () => {
      const message = { id: "msg-1", role: "user", content: "Hello", createdAt: "2024-01-01T00:00:00Z" };
      store.addMessage(sessionId, message);
      
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      store.updateMessage(sessionId, "non-existent", { content: "Updated" });
      
      expect(store.messagesBySession[sessionId][0].content).toBe("Hello");
      consoleSpy.mockRestore();
    });
  });

  describe("setSending", () => {
    it("should set sending state for session", () => {
      store.setSending(sessionId, true);
      
      expect(store.sendingBySession[sessionId]).toBe(true);
    });

    it("should update existing sending state", () => {
      store.setSending(sessionId, true);
      
      store.setSending(sessionId, false);
      
      expect(store.sendingBySession[sessionId]).toBe(false);
    });
  });

  describe("startStreamingMessage", () => {
    it("should create a new streaming message", () => {
      const message = store.startStreamingMessage(sessionId, "Initial content");
      
      expect(message.role).toBe("assistant");
      expect(message.status).toBe("streaming");
      expect(message.content).toBe("Initial content");
      expect(store.messagesBySession[sessionId]).toHaveLength(1);
      expect(store.streamingMessageIdBySession[sessionId]).toBe(message.id);
    });

    it("should throw error without sessionId", () => {
      expect(() => store.startStreamingMessage("")).toThrow("Missing sessionId");
    });
  });

  describe("appendStreamingMessage", () => {
    it("should append content to streaming message", () => {
      store.startStreamingMessage(sessionId, "Hello");
      
      store.appendStreamingMessage(sessionId, " World");
      
      expect(store.messagesBySession[sessionId][0].content).toBe("Hello World");
    });

    it("should not modify anything when no streaming message exists", () => {
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      
      store.appendStreamingMessage(sessionId, "content");
      
      expect(store.messagesBySession[sessionId]).toBeUndefined();
      consoleSpy.mockRestore();
    });

    it("should handle empty delta", () => {
      store.startStreamingMessage(sessionId, "Hello");
      
      store.appendStreamingMessage(sessionId, "");
      
      expect(store.messagesBySession[sessionId][0].content).toBe("Hello");
    });
  });

  describe("finishStreamingMessage", () => {
    it("should mark streaming message as completed", () => {
      store.startStreamingMessage(sessionId, "Content");
      
      store.finishStreamingMessage(sessionId);
      
      expect(store.messagesBySession[sessionId][0].status).toBe("completed");
      expect(store.streamingMessageIdBySession[sessionId]).toBeNull();
    });

    it("should handle finish without active streaming message", () => {
      store.finishStreamingMessage(sessionId);
      
      expect(store.streamingMessageIdBySession[sessionId]).toBeNull();
    });
  });

  describe("failStreamingMessage", () => {
    it("should mark streaming message as error with error message", () => {
      store.startStreamingMessage(sessionId, "Content");
      
      store.failStreamingMessage(sessionId, "Network error");
      
      expect(store.messagesBySession[sessionId][0].status).toBe("error");
      expect(store.messagesBySession[sessionId][0].error).toBe("Network error");
      expect(store.streamingMessageIdBySession[sessionId]).toBeNull();
    });

    it("should handle fail without active streaming message", () => {
      store.failStreamingMessage(sessionId, "Error");
      
      // Should not throw
      expect(store.streamingMessageIdBySession[sessionId]).toBeUndefined();
    });
  });

  describe("clearSession", () => {
    it("should clear all messages and state for session", () => {
      store.addMessage(sessionId, { id: "msg-1", role: "user", content: "Hello", createdAt: "2024-01-01T00:00:00Z" });
      store.setSending(sessionId, true);
      store.startStreamingMessage(sessionId);
      
      store.clearSession(sessionId);
      
      expect(store.messagesBySession[sessionId]).toEqual([]);
      expect(store.sendingBySession[sessionId]).toBe(false);
      expect(store.streamingMessageIdBySession[sessionId]).toBeNull();
    });
  });
});
