import { create } from "zustand";

import type { Id, Session } from "@/types";

const createId = (prefix: string) => {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `${prefix}_${crypto.randomUUID()}`;
  }

  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
};

const nowIso = () => new Date().toISOString();

export interface SessionState {
  sessions: Session[];
  currentSessionId: Id | null;
  createSession: (title?: string) => Session;
  switchSession: (id: Id) => boolean;
  deleteSession: (id: Id) => boolean;
  updateSession: (id: Id, updates: Partial<Pick<Session, "title" | "updatedAt">>) => boolean;
  setSessions: (sessions: Session[]) => void;
}

export const useSessionStore = create<SessionState>((set, get) => ({
  sessions: [],
  currentSessionId: null,
  createSession: (title) => {
    const sessionTitle = title?.trim() || `新会话 ${get().sessions.length + 1}`;
    const timestamp = nowIso();
    const session: Session = {
      id: createId("session"),
      title: sessionTitle,
      createdAt: timestamp,
      updatedAt: timestamp,
    };

    set((state) => ({
      sessions: [...state.sessions, session],
      currentSessionId: session.id,
    }));

    return session;
  },
  switchSession: (id) => {
    const exists = get().sessions.some((session) => session.id === id);
    if (!exists) {
      console.error(`Session not found: ${id}`);
      return false;
    }

    set({ currentSessionId: id });
    return true;
  },
  deleteSession: (id) => {
    const { sessions, currentSessionId } = get();
    const exists = sessions.some((session) => session.id === id);
    if (!exists) {
      console.warn(`Session not found: ${id}`);
      return false;
    }

    const remaining = sessions.filter((session) => session.id !== id);
    const nextCurrent =
      currentSessionId === id ? remaining[0]?.id ?? null : currentSessionId;

    set({
      sessions: remaining,
      currentSessionId: nextCurrent,
    });

    return true;
  },
  updateSession: (id, updates) => {
    const { sessions } = get();
    const index = sessions.findIndex((session) => session.id === id);
    if (index === -1) {
      console.warn(`Session not found: ${id}`);
      return false;
    }

    const nextSessions = [...sessions];
    nextSessions[index] = {
      ...nextSessions[index],
      ...updates,
      updatedAt: updates.updatedAt ?? nowIso(),
    };

    set({ sessions: nextSessions });
    return true;
  },
  setSessions: (sessions) => {
    set({
      sessions,
      currentSessionId: sessions[0]?.id ?? null,
    });
  },
}));
