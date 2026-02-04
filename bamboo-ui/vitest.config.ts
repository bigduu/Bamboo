import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-utils/setup.ts"],
    exclude: [
      "node_modules",
      "dist",
      ".next",
      "e2e/**", // Exclude E2E tests
    ],
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      thresholds: {
        statements: 80,
        branches: 80,
        functions: 80,
        lines: 80,
      },
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/**/*.d.ts",
        "src/test-utils/**",
        "src/**/__tests__/**",
        "src/**/*.test.{ts,tsx}",
      ],
    },
    // Isolate tests to prevent store state pollution
    isolate: true,
    // Reset mocks between tests
    mockReset: true,
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
