"use client";

import React, { createContext, useContext, useEffect, ReactNode } from "react";
import { useThemeStore, Theme } from "@/stores/themeStore";

interface ThemeContextType {
  theme: Theme;
  resolvedTheme: "light" | "dark";
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

interface ThemeProviderProps {
  children: ReactNode;
  defaultTheme?: Theme;
  enableSystem?: boolean;
}

export function ThemeProvider({
  children,
  defaultTheme = "system",
  enableSystem = true,
}: ThemeProviderProps) {
  const { theme, resolvedTheme, setTheme, toggleTheme, resolveTheme } =
    useThemeStore();

  // 初始化主题
  useEffect(() => {
    // 如果没有存储的主题，使用默认主题
    const storedTheme = localStorage.getItem("bamboo-theme");
    if (!storedTheme) {
      setTheme(defaultTheme);
    } else {
      resolveTheme();
    }
  }, [defaultTheme, setTheme, resolveTheme]);

  // 监听系统主题变化
  useEffect(() => {
    if (!enableSystem) return;

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = () => {
      if (theme === "system") {
        resolveTheme();
      }
    };

    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [theme, enableSystem, resolveTheme]);

  const value: ThemeContextType = {
    theme,
    resolvedTheme,
    setTheme,
    toggleTheme,
  };

  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    throw new Error("useTheme must be used within a ThemeProvider");
  }
  return context;
}
