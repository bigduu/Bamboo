import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { MessageBubble } from "../MessageBubble";
import type { Message } from "@/types";

// Mock the UI components
vi.mock("@/components/ui/avatar", () => ({
  Avatar: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div data-testid="avatar" className={className}>{children}</div>
  ),
  AvatarFallback: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="avatar-fallback">{children}</div>
  ),
}));

vi.mock("@/components/ui/card", () => ({
  Card: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div data-testid="card" className={className}>{children}</div>
  ),
  CardContent: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div data-testid="card-content" className={className}>{children}</div>
  ),
}));

vi.mock("lucide-react", () => ({
  User: () => <span data-testid="user-icon">User</span>,
  Bot: () => <span data-testid="bot-icon">Bot</span>,
}));

describe("MessageBubble", () => {
  const mockUserMessage: Message = {
    id: "msg-1",
    role: "user",
    content: "Hello, how are you?",
    createdAt: new Date().toISOString(),
  };

  const mockAssistantMessage: Message = {
    id: "msg-2",
    role: "assistant",
    content: "I'm doing great! How can I help you today?",
    createdAt: new Date().toISOString(),
  };

  it("should render user message correctly", () => {
    render(<MessageBubble message={{ ...mockUserMessage, timestamp: new Date() }} />);
    
    expect(screen.getByText("Hello, how are you?")).toBeInTheDocument();
    expect(screen.getByTestId("user-icon")).toBeInTheDocument();
  });

  it("should render assistant message correctly", () => {
    render(<MessageBubble message={{ ...mockAssistantMessage, timestamp: new Date() }} />);
    
    expect(screen.getByText("I'm doing great! How can I help you today?")).toBeInTheDocument();
    expect(screen.getByTestId("bot-icon")).toBeInTheDocument();
  });

  it("should display timestamp", () => {
    const timestamp = new Date("2024-01-15T10:30:00");
    render(<MessageBubble message={{ ...mockUserMessage, timestamp }} />);
    
    // Should display time in zh-CN format
    expect(screen.getByText(/10:30/)).toBeInTheDocument();
  });

  it("should render multiline content", () => {
    const multilineMessage: Message = {
      ...mockUserMessage,
      content: "Line 1\nLine 2\nLine 3",
      timestamp: new Date(),
    };
    
    render(<MessageBubble message={multilineMessage} />);
    
    expect(screen.getByText("Line 1")).toBeInTheDocument();
    expect(screen.getByText("Line 2")).toBeInTheDocument();
    expect(screen.getByText("Line 3")).toBeInTheDocument();
  });

  it("should render inline code", () => {
    const codeMessage: Message = {
      ...mockUserMessage,
      content: "Use the `console.log()` function",
      timestamp: new Date(),
    };
    
    render(<MessageBubble message={codeMessage} />);
    
    expect(screen.getByText(/console.log/)).toBeInTheDocument();
  });

  it("should render code blocks", () => {
    const codeBlockMessage: Message = {
      ...mockUserMessage,
      content: "```javascript\nconst x = 1;\n```",
      timestamp: new Date(),
    };
    
    render(<MessageBubble message={codeBlockMessage} />);
    
    expect(screen.getByText(/const x = 1;/)).toBeInTheDocument();
  });

  it("should apply correct styling for user messages", () => {
    const { container } = render(
      <MessageBubble message={{ ...mockUserMessage, timestamp: new Date() }} />
    );
    
    // User messages should have flex-row-reverse
    const messageContainer = container.firstChild;
    expect(messageContainer).toHaveClass("flex-row-reverse");
  });

  it("should apply correct styling for assistant messages", () => {
    const { container } = render(
      <MessageBubble message={{ ...mockAssistantMessage, timestamp: new Date() }} />
    );
    
    // Assistant messages should have flex-row
    const messageContainer = container.firstChild;
    expect(messageContainer).toHaveClass("flex-row");
  });
});
