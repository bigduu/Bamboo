import { create } from "zustand";

import type { Memory, SessionMemory } from "@/types";
import { getMemories, getSessionMemory } from "@/lib/api";

export interface MemoryState {
  memories: Memory[];
  sessionMemory: SessionMemory | null;
  loading: boolean;
  error: string | null;
  fetchMemories: () => Promise<void>;
  fetchSessionMemory: (sessionId: string) => Promise<void>;
}

export const useMemoryStore = create<MemoryState>((set) => ({
  memories: [],
  sessionMemory: null,
  loading: false,
  error: null,

  fetchMemories: async () => {
    set({ loading: true, error: null });
    try {
      const response = await getMemories();
      set({ memories: response.memories, loading: false });
    } catch (err) {
      set({ error: (err as Error).message, loading: false });
    }
  },

  fetchSessionMemory: async (sessionId) => {
    set({ loading: true, error: null });
    try {
      const response = await getSessionMemory(sessionId);
      set({ sessionMemory: response.session_memory, loading: false });
    } catch (err) {
      set({ error: (err as Error).message, loading: false });
    }
  },
}));
