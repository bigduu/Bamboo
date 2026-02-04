// Mock Zustand store for testing
import { act } from "@testing-library/react";

export const createMockStore = <T extends object>(initialState: T) => {
  let state = { ...initialState };
  const listeners = new Set<() => void>();

  const getState = () => state;
  const setState = (updater: (state: T) => T) => {
    state = updater(state);
    listeners.forEach((listener) => listener());
  };
  const subscribe = (listener: () => void) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  };

  return { getState, setState, subscribe };
};

// Helper to reset all Zustand stores between tests
export const resetStores = () => {
  // This will be implemented per store
};
