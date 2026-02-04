import { create } from "zustand";

import type { SystemPrompt } from "@/types";
import { getPrompts, createPrompt, updatePrompt, deletePrompt, setDefaultPrompt } from "@/lib/api";

export interface PromptState {
  prompts: SystemPrompt[];
  loading: boolean;
  error: string | null;
  fetchPrompts: () => Promise<void>;
  addPrompt: (prompt: Omit<SystemPrompt, "id">) => Promise<void>;
  editPrompt: (id: string, prompt: Partial<SystemPrompt>) => Promise<void>;
  removePrompt: (id: string) => Promise<void>;
  setDefault: (id: string) => Promise<void>;
}

export const usePromptStore = create<PromptState>((set, get) => ({
  prompts: [],
  loading: false,
  error: null,

  fetchPrompts: async () => {
    set({ loading: true, error: null });
    try {
      const response = await getPrompts();
      set({ prompts: response.prompts, loading: false });
    } catch (err) {
      set({ error: (err as Error).message, loading: false });
    }
  },

  addPrompt: async (prompt) => {
    try {
      const response = await createPrompt(prompt);
      set((state) => ({
        prompts: [...state.prompts, response.prompt],
      }));
    } catch (err) {
      set({ error: (err as Error).message });
    }
  },

  editPrompt: async (id, prompt) => {
    try {
      const response = await updatePrompt(id, prompt);
      set((state) => ({
        prompts: state.prompts.map((p) => (p.id === id ? response.prompt : p)),
      }));
    } catch (err) {
      set({ error: (err as Error).message });
    }
  },

  removePrompt: async (id) => {
    try {
      await deletePrompt(id);
      set((state) => ({
        prompts: state.prompts.filter((p) => p.id !== id),
      }));
    } catch (err) {
      set({ error: (err as Error).message });
    }
  },

  setDefault: async (id) => {
    try {
      const response = await setDefaultPrompt(id);
      set((state) => ({
        prompts: state.prompts.map((p) => ({
          ...p,
          is_default: p.id === id,
        })),
      }));
    } catch (err) {
      set({ error: (err as Error).message });
    }
  },
}));
