import { describe, it, expect } from "vitest";
import { cn } from "../utils";

describe("utils", () => {
  describe("cn", () => {
    it("should merge class names", () => {
      const result = cn("class1", "class2");
      expect(result).toBe("class1 class2");
    });

    it("should handle conditional classes", () => {
      const result = cn("base", true && "conditional-true", false && "conditional-false");
      expect(result).toBe("base conditional-true");
    });

    it("should handle undefined and null values", () => {
      const result = cn("base", undefined, null, "valid");
      expect(result).toBe("base valid");
    });

    it("should merge tailwind classes correctly", () => {
      const result = cn("px-2 py-1", "px-4");
      // tailwind-merge should keep the last conflicting class
      expect(result).toBe("py-1 px-4");
    });

    it("should handle empty input", () => {
      const result = cn();
      expect(result).toBe("");
    });

    it("should handle nested arrays", () => {
      const result = cn(["class1", "class2"], "class3");
      expect(result).toBe("class1 class2 class3");
    });

    it("should handle object syntax", () => {
      const result = cn({ "class1": true, "class2": false, "class3": true });
      expect(result).toBe("class1 class3");
    });
  });
});
