import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Theme = "light" | "dark" | "system";

interface ThemeState {
  theme: Theme;
  resolvedTheme: "light" | "dark";
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  resolveTheme: () => void;
}

const getSystemTheme = (): "light" | "dark" => {
  if (typeof window === "undefined") return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
};

const applyTheme = (theme: Theme, resolved: "light" | "dark") => {
  if (typeof document === "undefined") return;

  const root = document.documentElement;

  if (theme === "system") {
    root.classList.remove("light", "dark");
    root.classList.add(resolved);
  } else {
    root.classList.remove("light", "dark");
    root.classList.add(theme);
  }

  // 添加过渡动画
  root.style.transition = "background-color 0.3s ease, color 0.3s ease";
};

export const useThemeStore = create<ThemeState>()(
  persist(
    (set, get) => ({
      theme: "system",
      resolvedTheme: "light",

      setTheme: (theme) => {
        const resolved = theme === "system" ? getSystemTheme() : theme;
        set({ theme, resolvedTheme: resolved });
        applyTheme(theme, resolved);
      },

      toggleTheme: () => {
        const { resolvedTheme } = get();
        const newTheme = resolvedTheme === "light" ? "dark" : "light";
        set({ theme: newTheme, resolvedTheme: newTheme });
        applyTheme(newTheme, newTheme);
      },

      resolveTheme: () => {
        const { theme } = get();
        const resolved = theme === "system" ? getSystemTheme() : theme;
        set({ resolvedTheme: resolved });
        applyTheme(theme, resolved);
      },
    }),
    {
      name: "bamboo-theme",
      onRehydrateStorage: () => (state) => {
        if (state) {
          const resolved =
            state.theme === "system" ? getSystemTheme() : state.theme;
          state.resolvedTheme = resolved;
          applyTheme(state.theme, resolved);
        }
      },
    }
  )
);

// 监听系统主题变化
if (typeof window !== "undefined") {
  const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  mediaQuery.addEventListener("change", (e) => {
    const { theme, resolveTheme } = useThemeStore.getState();
    if (theme === "system") {
      resolveTheme();
    }
  });
}
