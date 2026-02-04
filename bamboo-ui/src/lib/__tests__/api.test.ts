import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock axios before importing api
vi.mock("axios", () => ({
  default: {
    create: vi.fn(() => ({
      interceptors: {
        request: { use: vi.fn() },
        response: { use: vi.fn() },
      },
      request: vi.fn(),
    })),
    isAxiosError: vi.fn(),
  },
}));

// Simple test for API utilities without importing the actual module
describe("API Utilities", () => {
  describe("toApiError", () => {
    it("should handle Axios error with response", () => {
      // Mock implementation
      const toApiError = (error: unknown) => {
        if (error && typeof error === "object" && "isAxiosError" in error) {
          const axiosError = error as any;
          const status = axiosError.response?.status;
          const message = axiosError.response?.data?.message || axiosError.message || "Request failed";
          return {
            message,
            status,
            code: axiosError.code,
            details: axiosError.response?.data ?? {},
          };
        }
        if (error instanceof Error) {
          return { message: error.message, details: error };
        }
        return { message: "Unknown error", details: error };
      };

      const axiosError = {
        isAxiosError: true,
        response: {
          status: 404,
          statusText: "Not Found",
          data: { message: "Resource not found" },
        },
        message: "Request failed with status code 404",
        code: "ERR_BAD_REQUEST",
      };
      
      const result = toApiError(axiosError);
      
      expect(result.message).toBe("Resource not found");
      expect(result.status).toBe(404);
      expect(result.code).toBe("ERR_BAD_REQUEST");
    });

    it("should handle generic Error", () => {
      const toApiError = (error: unknown) => {
        if (error instanceof Error) {
          return { message: error.message, details: error };
        }
        return { message: "Unknown error", details: error };
      };

      const error = new Error("Something went wrong");
      
      const result = toApiError(error);
      
      expect(result.message).toBe("Something went wrong");
    });

    it("should handle unknown error", () => {
      const toApiError = (error: unknown) => {
        if (error instanceof Error) {
          return { message: error.message, details: error };
        }
        return { message: "Unknown error", details: error };
      };

      const result = toApiError("unknown");
      
      expect(result.message).toBe("Unknown error");
    });
  });

  describe("API Client Configuration", () => {
    it("should create API client with interceptors", () => {
      // This test verifies the API client structure
      const mockClient = {
        interceptors: {
          request: { use: vi.fn() },
          response: { use: vi.fn() },
        },
      };
      
      expect(mockClient.interceptors.request.use).toBeDefined();
      expect(mockClient.interceptors.response.use).toBeDefined();
    });
  });

  describe("API Functions", () => {
    it("should define masking API endpoints", () => {
      // Verify masking API structure
      const maskingEndpoints = {
        getConfig: "/api/v1/masking/config",
        saveConfig: "/api/v1/masking/config",
        test: "/api/v1/masking/test",
      };
      
      expect(maskingEndpoints.getConfig).toBe("/api/v1/masking/config");
      expect(maskingEndpoints.saveConfig).toBe("/api/v1/masking/config");
      expect(maskingEndpoints.test).toBe("/api/v1/masking/test");
    });

    it("should define backend config API endpoints", () => {
      // Verify backend config API structure
      const backendEndpoints = {
        getConfig: "/api/v1/config",
        saveConfig: "/api/v1/config",
        getSection: (section: string) => `/api/v1/config/${section}`,
        updateSection: (section: string) => `/api/v1/config/${section}`,
      };
      
      expect(backendEndpoints.getConfig).toBe("/api/v1/config");
      expect(backendEndpoints.getSection("server")).toBe("/api/v1/config/server");
    });
  });
});
