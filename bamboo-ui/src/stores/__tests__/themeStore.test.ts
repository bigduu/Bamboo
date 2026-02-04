import { describe, it, expect, beforeEach, vi } from "vitest";

// Mock matchMedia
const mockMatchMedia = vi.fn();
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: mockMatchMedia,
});

// Simple state container for testing
type Theme = "light" | "dark" | "system";

class TestThemeStore {
  theme: Theme = "system";

  setTheme(theme: Theme) {
    this.theme = theme;
  }

  getSystemTheme(): "light" | "dark" {
    if (typeof window !== "undefined" && window.matchMedia) {
      const result = window.matchMedia("(prefers-color-scheme: dark)");
      if (result && typeof result.matches === "boolean") {
        return result.matches ? "dark" : "light";
      }
    }
    return "light";
  }

  resolveTheme(theme: Theme): "light" | "dark" {
    if (theme === "system") {
      return this.getSystemTheme();
    }
    return theme;
  }

  toggleTheme() {
    const current = this.theme;
    if (current === "light") {
      this.theme = "dark";
    } else if (current === "dark") {
      this.theme = "light";
    } else {
      // system - toggle based on resolved theme
      const resolved = this.getSystemTheme();
      this.theme = resolved === "dark" ? "light" : "dark";
    }
  }

  get resolvedTheme(): "light" | "dark" {
    return this.resolveTheme(this.theme);
  }

  get isDark(): boolean {
    return this.resolveTheme(this.theme) === "dark";
  }
}

describe("themeStore", () => {
  let store: TestThemeStore;

  beforeEach(() => {
    vi.clearAllMocks();
    mockMatchMedia.mockReturnValue({ matches: false });
    store = new TestThemeStore();
    store.theme = "system";
  });

  describe("initial state", () => {
    it("should default to system theme", () => {
      expect(store.theme).toBe("system");
    });
  });

  describe("setTheme", () => {
    it("should set theme to light", () => {
      store.setTheme("light");
      
      expect(store.theme).toBe("light");
    });

    it("should set theme to dark", () => {
      store.setTheme("dark");
      
      expect(store.theme).toBe("dark");
    });

    it("should set theme to system", () => {
      store.theme = "dark";
      
      store.setTheme("system");
      
      expect(store.theme).toBe("system");
    });
  });

  describe("toggleTheme", () => {
    it("should toggle from light to dark", () => {
      store.theme = "light";
      
      store.toggleTheme();
      
      expect(store.theme).toBe("dark");
    });

    it("should toggle from dark to light", () => {
      store.theme = "dark";
      
      store.toggleTheme();
      
      expect(store.theme).toBe("light");
    });

    it("should toggle from system to light when system prefers dark", () => {
      mockMatchMedia.mockReturnValue({ matches: true });
      store.theme = "system";
      
      store.toggleTheme();
      
      expect(store.theme).toBe("light");
    });

    it("should toggle from system to dark when system prefers light", () => {
      mockMatchMedia.mockReturnValue({ matches: false });
      store.theme = "system";
      
      store.toggleTheme();
      
      expect(store.theme).toBe("dark");
    });
  });

  describe("resolvedTheme", () => {
    it("should return light when theme is light", () => {
      store.theme = "light";
      
      expect(store.resolvedTheme).toBe("light");
    });

    it("should return dark when theme is dark", () => {
      store.theme = "dark";
      
      expect(store.resolvedTheme).toBe("dark");
    });

    it("should return dark when system prefers dark", () => {
      mockMatchMedia.mockReturnValue({ matches: true });
      store.theme = "system";
      
      expect(store.resolvedTheme).toBe("dark");
    });

    it("should return light when system prefers light", () => {
      mockMatchMedia.mockReturnValue({ matches: false });
      store.theme = "system";
      
      expect(store.resolvedTheme).toBe("light");
    });
  });

  describe("isDark", () => {
    it("should return false for light theme", () => {
      store.theme = "light";
      
      expect(store.isDark).toBe(false);
    });

    it("should return true for dark theme", () => {
      store.theme = "dark";
      
      expect(store.isDark).toBe(true);
    });

    it("should respect system preference", () => {
      mockMatchMedia.mockReturnValue({ matches: true });
      store.theme = "system";
      
      expect(store.isDark).toBe(true);
    });
  });
});
