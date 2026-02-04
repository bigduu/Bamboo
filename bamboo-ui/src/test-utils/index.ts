import { render as rtlRender, RenderOptions } from "@testing-library/react";
import React, { ReactElement, ReactNode } from "react";

// Custom render with providers if needed
interface CustomRenderOptions extends Omit<RenderOptions, "wrapper"> {
  // Add any custom options here
}

function AllProviders({ children }: { children: ReactNode }) {
  // Add global providers here (ThemeProvider, etc.)
  return React.createElement(React.Fragment, null, children);
}

export function render(ui: ReactElement, options?: CustomRenderOptions) {
  return rtlRender(ui, { wrapper: AllProviders, ...options });
}

// Re-export everything from testing-library except render
export * from "@testing-library/react";
// Override the render export
export { render as customRender };
