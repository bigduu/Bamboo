import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { useState } from "react";

// Mock theme toggle component
type Theme = "light" | "dark" | "system";

const getSystemTheme = (): "light" | "dark" => {
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "light";
};

const MockThemeToggle = () => {
  const [theme, setTheme] = useState<Theme>("system");
  
  const resolvedTheme = theme === "system" ? getSystemTheme() : theme;
  const isDark = resolvedTheme === "dark";
  
  const toggleTheme = () => {
    if (theme === "light") {
      setTheme("dark");
    } else if (theme === "dark") {
      setTheme("light");
    } else {
      // system - toggle based on resolved theme
      const resolved = getSystemTheme();
      setTheme(resolved === "dark" ? "light" : "dark");
    }
  };
  
  return (
    <div data-testid="theme-toggle">
      <span data-testid="current-theme">{theme}</span>
      <span data-testid="resolved-theme">{resolvedTheme}</span>
      <span data-testid="is-dark">{isDark ? "true" : "false"}</span>
      
      <button data-testid="toggle-btn" onClick={toggleTheme}>Toggle</button>
      <button data-testid="set-light" onClick={() => setTheme("light")}>Light</button>
      <button data-testid="set-dark" onClick={() => setTheme("dark")}>Dark</button>
      <button data-testid="set-system" onClick={() => setTheme("system")}>System</button>
    </div>
  );
};

// Mock matchMedia
const mockMatchMedia = vi.fn();
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: mockMatchMedia,
});

describe("Theme Switching Integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockMatchMedia.mockReturnValue({ matches: false });
  });

  it("should default to system theme", () => {
    render(<MockThemeToggle />);
    
    expect(screen.getByTestId("current-theme")).toHaveTextContent("system");
  });

  it("should switch to light theme", () => {
    render(<MockThemeToggle />);
    
    fireEvent.click(screen.getByTestId("set-light"));
    
    expect(screen.getByTestId("current-theme")).toHaveTextContent("light");
    expect(screen.getByTestId("resolved-theme")).toHaveTextContent("light");
    expect(screen.getByTestId("is-dark")).toHaveTextContent("false");
  });

  it("should switch to dark theme", () => {
    render(<MockThemeToggle />);
    
    fireEvent.click(screen.getByTestId("set-dark"));
    
    expect(screen.getByTestId("current-theme")).toHaveTextContent("dark");
    expect(screen.getByTestId("resolved-theme")).toHaveTextContent("dark");
    expect(screen.getByTestId("is-dark")).toHaveTextContent("true");
  });

  it("should respect system preference when theme is system", () => {
    mockMatchMedia.mockReturnValue({ matches: true }); // Dark mode
    
    render(<MockThemeToggle />);
    
    // Current theme is system
    expect(screen.getByTestId("current-theme")).toHaveTextContent("system");
    // But resolved should be dark
    expect(screen.getByTestId("resolved-theme")).toHaveTextContent("dark");
    expect(screen.getByTestId("is-dark")).toHaveTextContent("true");
  });

  it("should toggle from light to dark", () => {
    render(<MockThemeToggle />);
    
    // Set to light first
    fireEvent.click(screen.getByTestId("set-light"));
    expect(screen.getByTestId("current-theme")).toHaveTextContent("light");
    
    // Toggle
    fireEvent.click(screen.getByTestId("toggle-btn"));
    
    expect(screen.getByTestId("current-theme")).toHaveTextContent("dark");
  });

  it("should toggle from dark to light", () => {
    render(<MockThemeToggle />);
    
    // Set to dark first
    fireEvent.click(screen.getByTestId("set-dark"));
    expect(screen.getByTestId("current-theme")).toHaveTextContent("dark");
    
    // Toggle
    fireEvent.click(screen.getByTestId("toggle-btn"));
    
    expect(screen.getByTestId("current-theme")).toHaveTextContent("light");
  });

  it("should toggle from system to opposite of system preference", () => {
    mockMatchMedia.mockReturnValue({ matches: true }); // System prefers dark
    
    render(<MockThemeToggle />);
    
    // Currently system (resolved to dark)
    expect(screen.getByTestId("current-theme")).toHaveTextContent("system");
    expect(screen.getByTestId("resolved-theme")).toHaveTextContent("dark");
    
    // Toggle should go to light (opposite of system)
    fireEvent.click(screen.getByTestId("toggle-btn"));
    
    expect(screen.getByTestId("current-theme")).toHaveTextContent("light");
  });
});
