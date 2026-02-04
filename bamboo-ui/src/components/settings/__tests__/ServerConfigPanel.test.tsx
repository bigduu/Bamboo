import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ServerConfigPanel } from "../ServerConfigPanel";

// Mock the config store
const mockSetApiUrl = vi.fn();
const mockSetWsUrl = vi.fn();
const mockTestConnection = vi.fn();

vi.mock("@/stores/configStore", () => ({
  useConfigStore: () => ({
    config: {
      apiUrl: "http://localhost:3000",
      wsUrl: "ws://localhost:18790",
    },
    setApiUrl: mockSetApiUrl,
    setWsUrl: mockSetWsUrl,
    testConnection: mockTestConnection,
  }),
}));

// Mock UI components
vi.mock("@/components/ui/button", () => ({
  Button: ({ children, onClick, disabled }: any) => (
    <button data-testid="button" onClick={onClick} disabled={disabled}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/input", () => ({
  Input: ({ id, value, onChange, disabled, placeholder }: any) => (
    <input
      data-testid={id}
      value={value}
      onChange={onChange}
      disabled={disabled}
      placeholder={placeholder}
    />
  ),
}));

vi.mock("@/components/ui/label", () => ({
  Label: ({ children, htmlFor }: any) => (
    <label data-testid={`label-${htmlFor}`}>{children}</label>
  ),
}));

vi.mock("@/components/ui/card", () => ({
  Card: ({ children }: any) => <div data-testid="card">{children}</div>,
  CardHeader: ({ children }: any) => <div data-testid="card-header">{children}</div>,
  CardTitle: ({ children }: any) => <div data-testid="card-title">{children}</div>,
  CardDescription: ({ children }: any) => <div data-testid="card-description">{children}</div>,
  CardContent: ({ children }: any) => <div data-testid="card-content">{children}</div>,
}));

vi.mock("@/components/ui/badge", () => ({
  Badge: ({ children }: any) => <span data-testid="badge">{children}</span>,
}));

vi.mock("lucide-react", () => ({
  Server: () => <span>Server</span>,
  Wifi: () => <span>Wifi</span>,
  CheckCircle: () => <span data-testid="check-icon">Check</span>,
  XCircle: () => <span data-testid="x-icon">X</span>,
  Loader2: () => <span data-testid="loader-icon">Loading</span>,
}));

describe("ServerConfigPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render card with title", () => {
    render(<ServerConfigPanel />);
    
    expect(screen.getByTestId("card")).toBeInTheDocument();
    expect(screen.getByTestId("card-title")).toBeInTheDocument();
  });

  it("should display API URL input", () => {
    render(<ServerConfigPanel />);
    
    expect(screen.getByTestId("api-url")).toBeInTheDocument();
    expect(screen.getByTestId("api-url")).toHaveValue("http://localhost:3000");
  });

  it("should display WebSocket URL input", () => {
    render(<ServerConfigPanel />);
    
    expect(screen.getByTestId("ws-url")).toBeInTheDocument();
    expect(screen.getByTestId("ws-url")).toHaveValue("ws://localhost:18790");
  });

  it("should update API URL on change", () => {
    render(<ServerConfigPanel />);
    
    const input = screen.getByTestId("api-url");
    fireEvent.change(input, { target: { value: "http://new-api.com" } });
    
    expect(mockSetApiUrl).toHaveBeenCalledWith("http://new-api.com");
  });

  it("should update WebSocket URL on change", () => {
    render(<ServerConfigPanel />);
    
    const input = screen.getByTestId("ws-url");
    fireEvent.change(input, { target: { value: "ws://new-ws.com" } });
    
    expect(mockSetWsUrl).toHaveBeenCalledWith("ws://new-ws.com");
  });

  it("should test connection when button is clicked", async () => {
    mockTestConnection.mockResolvedValue({ success: true, latency: 50 });
    
    render(<ServerConfigPanel />);
    
    const testButton = screen.getByText(/Test Connection/i);
    fireEvent.click(testButton);
    
    await waitFor(() => {
      expect(mockTestConnection).toHaveBeenCalled();
    });
  });

  it("should display success message on successful connection test", async () => {
    mockTestConnection.mockResolvedValue({ success: true, latency: 50 });
    
    render(<ServerConfigPanel />);
    
    const testButton = screen.getByText(/Test Connection/i);
    fireEvent.click(testButton);
    
    await waitFor(() => {
      expect(screen.getByText(/Connected \(50ms\)/i)).toBeInTheDocument();
    });
  });

  it("should display error message on failed connection test", async () => {
    mockTestConnection.mockResolvedValue({ success: false, error: "Connection refused" });
    
    render(<ServerConfigPanel />);
    
    const testButton = screen.getByText(/Test Connection/i);
    fireEvent.click(testButton);
    
    await waitFor(() => {
      expect(screen.getByText(/Connection refused/i)).toBeInTheDocument();
    });
  });

  it("should show loading state during connection test", async () => {
    mockTestConnection.mockImplementation(() => new Promise(() => {})); // Never resolves
    
    render(<ServerConfigPanel />);
    
    const testButton = screen.getByText(/Test Connection/i);
    fireEvent.click(testButton);
    
    expect(screen.getByTestId("loader-icon")).toBeInTheDocument();
  });

  it("should disable test button during connection test", async () => {
    mockTestConnection.mockImplementation(() => new Promise(() => {}));
    
    render(<ServerConfigPanel />);
    
    const testButton = screen.getByText(/Test Connection/i);
    fireEvent.click(testButton);
    
    expect(screen.getByTestId("button")).toBeDisabled();
  });
});
