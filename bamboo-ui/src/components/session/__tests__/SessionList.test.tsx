import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SessionList } from "../SessionList";
import type { Session } from "@/types";

// Mock SessionItem component
vi.mock("../SessionItem", () => ({
  SessionItem: ({ session, isActive, onClick }: any) => (
    <div
      data-testid={`session-item-${session.id}`}
      data-active={isActive}
      onClick={onClick}
    >
      {session.title}
    </div>
  ),
}));

describe("SessionList", () => {
  const mockSessions: Session[] = [
    {
      id: "1",
      title: "Session 1",
      createdAt: "2024-01-01T00:00:00Z",
      updatedAt: "2024-01-01T00:00:00Z",
    },
    {
      id: "2",
      title: "Session 2",
      createdAt: "2024-01-02T00:00:00Z",
      updatedAt: "2024-01-02T00:00:00Z",
    },
  ];

  const mockOnSelectSession = vi.fn();

  beforeEach(() => {
    mockOnSelectSession.mockClear();
  });

  it("should render session list title", () => {
    render(<SessionList sessions={mockSessions} onSelectSession={mockOnSelectSession} />);
    
    expect(screen.getByText("会话历史")).toBeInTheDocument();
  });

  it("should render sessions", () => {
    render(
      <SessionList 
        sessions={mockSessions}
        onSelectSession={mockOnSelectSession}
      />
    );
    
    expect(screen.getByText("Session 1")).toBeInTheDocument();
    expect(screen.getByText("Session 2")).toBeInTheDocument();
  });

  it("should mark active session correctly", () => {
    render(
      <SessionList 
        sessions={mockSessions}
        activeSessionId="1"
        onSelectSession={mockOnSelectSession}
      />
    );
    
    const session1 = screen.getByTestId("session-item-1");
    expect(session1).toHaveAttribute("data-active", "true");
  });

  it("should call onSelectSession when session is clicked", () => {
    render(
      <SessionList 
        sessions={mockSessions}
        onSelectSession={mockOnSelectSession}
      />
    );
    
    const session1 = screen.getByTestId("session-item-1");
    fireEvent.click(session1);
    
    expect(mockOnSelectSession).toHaveBeenCalledWith("1");
  });

  it("should handle empty sessions array", () => {
    render(<SessionList sessions={[]} onSelectSession={mockOnSelectSession} />
    );
    
    expect(screen.getByText("会话历史")).toBeInTheDocument();
    // No session items should be rendered
    expect(screen.queryByTestId(/session-item-/)).not.toBeInTheDocument();
  });
});
