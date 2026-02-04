"use client";

import { useMemo } from "react";

import { useSessionStore } from "@/stores/sessionStore";
import type { Session } from "@/types";

export const useSession = () => {
  const sessions = useSessionStore((state) => state.sessions);
  const currentSessionId = useSessionStore((state) => state.currentSessionId);
  const createSession = useSessionStore((state) => state.createSession);
  const switchSession = useSessionStore((state) => state.switchSession);
  const deleteSession = useSessionStore((state) => state.deleteSession);
  const updateSession = useSessionStore((state) => state.updateSession);

  const currentSession = useMemo<Session | null>(() => {
    return sessions.find((session) => session.id === currentSessionId) ?? null;
  }, [sessions, currentSessionId]);

  return {
    sessions,
    currentSessionId,
    currentSession,
    createSession,
    switchSession,
    deleteSession,
    updateSession,
  };
};
