import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { InputArea } from "../InputArea";

// Mock the UI components
vi.mock("@/components/ui/button", () => ({
  Button: ({ children, onClick, disabled, type }: any) => (
    <button data-testid="send-button" onClick={onClick} disabled={disabled} type={type}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/input", () => ({
  Input: ({ value, onChange, onKeyDown, placeholder, disabled, className }: any) => (
    <input
      data-testid="message-input"
      value={value}
      onChange={onChange}
      onKeyDown={onKeyDown}
      placeholder={placeholder}
      disabled={disabled}
      className={className}
    />
  ),
}));

vi.mock("lucide-react", () => ({
  Send: () => <span data-testid="send-icon">Send</span>,
  Loader2: () => <span data-testid="loader-icon">Loading</span>,
}));

describe("InputArea", () => {
  const mockOnSendMessage = vi.fn();

  beforeEach(() => {
    mockOnSendMessage.mockClear();
  });

  it("should render input and button", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    expect(screen.getByTestId("message-input")).toBeInTheDocument();
    expect(screen.getByTestId("send-button")).toBeInTheDocument();
  });

  it("should display custom placeholder", () => {
    render(
      <InputArea 
        onSendMessage={mockOnSendMessage} 
        placeholder="Custom placeholder"
      />
    );
    
    expect(screen.getByPlaceholderText("Custom placeholder")).toBeInTheDocument();
  });

  it("should update input value on change", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    const input = screen.getByTestId("message-input");
    fireEvent.change(input, { target: { value: "Hello" } });
    
    expect(input).toHaveValue("Hello");
  });

  it("should call onSendMessage when form is submitted", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    const input = screen.getByTestId("message-input");
    fireEvent.change(input, { target: { value: "Test message" } });
    
    const form = input.closest("form");
    fireEvent.submit(form!);
    
    expect(mockOnSendMessage).toHaveBeenCalledWith("Test message");
  });

  it("should clear input after sending message", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    const input = screen.getByTestId("message-input");
    fireEvent.change(input, { target: { value: "Test message" } });
    
    const form = input.closest("form");
    fireEvent.submit(form!);
    
    expect(input).toHaveValue("");
  });

  it("should not send empty messages", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    const input = screen.getByTestId("message-input");
    fireEvent.change(input, { target: { value: "   " } });
    
    const form = input.closest("form");
    fireEvent.submit(form!);
    
    expect(mockOnSendMessage).not.toHaveBeenCalled();
  });

  it("should send message on Enter key press", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    const input = screen.getByTestId("message-input");
    fireEvent.change(input, { target: { value: "Test message" } });
    fireEvent.keyDown(input, { key: "Enter" });
    
    expect(mockOnSendMessage).toHaveBeenCalledWith("Test message");
  });

  it("should not send message on Shift+Enter", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    const input = screen.getByTestId("message-input");
    fireEvent.change(input, { target: { value: "Test message" } });
    fireEvent.keyDown(input, { key: "Enter", shiftKey: true });
    
    expect(mockOnSendMessage).not.toHaveBeenCalled();
  });

  it("should disable input and button when loading", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} isLoading={true} />);
    
    expect(screen.getByTestId("message-input")).toBeDisabled();
    expect(screen.getByTestId("send-button")).toBeDisabled();
  });

  it("should show loading icon when isLoading is true", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} isLoading={true} />);
    
    expect(screen.getByTestId("loader-icon")).toBeInTheDocument();
  });

  it("should show send icon when not loading", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} isLoading={false} />);
    
    expect(screen.getByTestId("send-icon")).toBeInTheDocument();
  });

  it("should disable button when input is empty", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    expect(screen.getByTestId("send-button")).toBeDisabled();
  });

  it("should enable button when input has content", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    const input = screen.getByTestId("message-input");
    fireEvent.change(input, { target: { value: "Hello" } });
    
    expect(screen.getByTestId("send-button")).not.toBeDisabled();
  });

  it("should display helper text", () => {
    render(<InputArea onSendMessage={mockOnSendMessage} />);
    
    expect(screen.getByText(/按 Enter 发送消息/)).toBeInTheDocument();
  });

  it("should apply custom className", () => {
    const { container } = render(
      <InputArea onSendMessage={mockOnSendMessage} className="custom-class" />
    );
    
    expect(container.firstChild).toHaveClass("custom-class");
  });
});
