import { describe, it, expect, beforeEach, vi } from "vitest";

// Simple state container for testing
class TestSessionStore {
  sessions: Array<{ id: string; title: string; createdAt: string; updatedAt: string }> = [];
  currentSessionId: string | null = null;

  createId(prefix: string) {
    return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
  }

  nowIso() {
    return new Date().toISOString();
  }

  createSession(title?: string) {
    const sessionTitle = title?.trim() || `新会话 ${this.sessions.length + 1}`;
    const timestamp = this.nowIso();
    const session = {
      id: this.createId("session"),
      title: sessionTitle,
      createdAt: timestamp,
      updatedAt: timestamp,
    };

    this.sessions = [...this.sessions, session];
    this.currentSessionId = session.id;

    return session;
  }

  switchSession(id: string) {
    const exists = this.sessions.some((session) => session.id === id);
    if (!exists) {
      console.error(`Session not found: ${id}`);
      return false;
    }

    this.currentSessionId = id;
    return true;
  }

  deleteSession(id: string) {
    const exists = this.sessions.some((session) => session.id === id);
    if (!exists) {
      console.warn(`Session not found: ${id}`);
      return false;
    }

    const remaining = this.sessions.filter((session) => session.id !== id);
    const nextCurrent =
      this.currentSessionId === id ? remaining[0]?.id ?? null : this.currentSessionId;

    this.sessions = remaining;
    this.currentSessionId = nextCurrent;

    return true;
  }

  updateSession(id: string, updates: Partial<{ title: string; updatedAt: string }>) {
    const index = this.sessions.findIndex((session) => session.id === id);
    if (index === -1) {
      console.warn(`Session not found: ${id}`);
      return false;
    }

    const nextSessions = [...this.sessions];
    nextSessions[index] = {
      ...nextSessions[index],
      ...updates,
      updatedAt: updates.updatedAt ?? this.nowIso(),
    };

    this.sessions = nextSessions;
    return true;
  }

  setSessions(sessions: typeof this.sessions) {
    this.sessions = sessions;
    this.currentSessionId = sessions[0]?.id ?? null;
  }
}

describe("sessionStore", () => {
  let store: TestSessionStore;

  beforeEach(() => {
    store = new TestSessionStore();
  });

  describe("createSession", () => {
    it("should create a new session with default title", () => {
      const session = store.createSession();
      
      expect(session).toBeDefined();
      expect(session.title).toBe("新会话 1");
      expect(session.id).toMatch(/^session_/);
      expect(store.sessions).toHaveLength(1);
      expect(store.currentSessionId).toBe(session.id);
    });

    it("should create a session with custom title", () => {
      const session = store.createSession("Custom Title");
      
      expect(session.title).toBe("Custom Title");
    });

    it("should trim whitespace from title", () => {
      const session = store.createSession("  Custom Title  ");
      
      expect(session.title).toBe("Custom Title");
    });

    it("should auto-increment session numbers", () => {
      store.createSession();
      store.createSession();
      const third = store.createSession();
      
      expect(third.title).toBe("新会话 3");
    });
  });

  describe("switchSession", () => {
    it("should switch to an existing session", () => {
      const session1 = store.createSession("Session 1");
      const session2 = store.createSession("Session 2");
      
      const result = store.switchSession(session1.id);
      
      expect(result).toBe(true);
      expect(store.currentSessionId).toBe(session1.id);
    });

    it("should return false for non-existent session", () => {
      store.createSession();
      
      const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
      const result = store.switchSession("non-existent-id");
      
      expect(result).toBe(false);
      consoleSpy.mockRestore();
    });
  });

  describe("deleteSession", () => {
    it("should delete an existing session", () => {
      const session = store.createSession();
      
      const result = store.deleteSession(session.id);
      
      expect(result).toBe(true);
      expect(store.sessions).toHaveLength(0);
    });

    it("should switch to another session when deleting current session", () => {
      const session1 = store.createSession("Session 1");
      const session2 = store.createSession("Session 2");
      store.switchSession(session1.id);
      
      store.deleteSession(session1.id);
      
      expect(store.currentSessionId).toBe(session2.id);
    });

    it("should set currentSessionId to null when deleting last session", () => {
      const session = store.createSession();
      
      store.deleteSession(session.id);
      
      expect(store.currentSessionId).toBeNull();
    });

    it("should return false for non-existent session", () => {
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      const result = store.deleteSession("non-existent-id");
      
      expect(result).toBe(false);
      consoleSpy.mockRestore();
    });
  });

  describe("updateSession", () => {
    it("should update session title", () => {
      const session = store.createSession("Original Title");
      
      const result = store.updateSession(session.id, { title: "Updated Title" });
      
      expect(result).toBe(true);
      expect(store.sessions[0].title).toBe("Updated Title");
    });

    it("should update updatedAt timestamp", () => {
      const session = store.createSession();
      const originalUpdatedAt = session.updatedAt;
      
      // Wait a bit to ensure timestamp changes
      vi.useFakeTimers();
      vi.advanceTimersByTime(1000);
      
      store.updateSession(session.id, { title: "Updated" });
      
      expect(store.sessions[0].updatedAt).not.toBe(originalUpdatedAt);
      vi.useRealTimers();
    });

    it("should return false for non-existent session", () => {
      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
      const result = store.updateSession("non-existent-id", { title: "Updated" });
      
      expect(result).toBe(false);
      consoleSpy.mockRestore();
    });
  });

  describe("setSessions", () => {
    it("should set all sessions and update current session", () => {
      const sessions = [
        { id: "s1", title: "Session 1", createdAt: "2024-01-01", updatedAt: "2024-01-01" },
        { id: "s2", title: "Session 2", createdAt: "2024-01-02", updatedAt: "2024-01-02" },
      ];
      
      store.setSessions(sessions);
      
      expect(store.sessions).toHaveLength(2);
      expect(store.currentSessionId).toBe("s1");
    });

    it("should set currentSessionId to null for empty sessions array", () => {
      store.setSessions([]);
      
      expect(store.sessions).toHaveLength(0);
      expect(store.currentSessionId).toBeNull();
    });
  });
});
