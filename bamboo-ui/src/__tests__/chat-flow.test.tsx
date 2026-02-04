import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { useState } from "react";

// Simple mock components for integration testing
const MockChatContainer = () => {
  const [sessions, setSessions] = useState<Array<{ id: string; title: string }>>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Record<string, Array<{ role: string; content: string }>>>({});
  
  const createSession = () => {
    const id = `session_${Date.now()}`;
    const newSession = { id, title: `新会话 ${sessions.length + 1}` };
    setSessions([...sessions, newSession]);
    setCurrentSessionId(id);
    setMessages({ ...messages, [id]: [] });
    return newSession;
  };
  
  const switchSession = (id: string) => {
    setCurrentSessionId(id);
  };
  
  const sendMessage = (content: string) => {
    if (!currentSessionId) return;
    
    const sessionMessages = messages[currentSessionId] || [];
    setMessages({
      ...messages,
      [currentSessionId]: [...sessionMessages, { role: "user", content }],
    });
  };
  
  return (
    <div data-testid="chat-container">
      <div data-testid="session-list">
        {sessions.map(session => (
          <button
            key={session.id}
            data-testid={`session-${session.id}`}
            data-active={session.id === currentSessionId}
            onClick={() => switchSession(session.id)}
          >
            {session.title}
          </button>
        ))}
        <button data-testid="new-session" onClick={createSession}>New Session</button>
      </div>
      
      <div data-testid="message-list">
        {(currentSessionId ? messages[currentSessionId] || [] : []).map((msg, idx) => (
          <div key={idx} data-testid={`message-${msg.role}`}>{msg.content}</div>
        ))}
      </div>
      
      <input
        data-testid="message-input"
        onKeyDown={(e) => {
          if (e.key === "Enter" && e.currentTarget.value.trim()) {
            sendMessage(e.currentTarget.value.trim());
            e.currentTarget.value = "";
          }
        }}
      />
    </div>
  );
};

describe("Chat Flow Integration", () => {
  it("should create a new session and send messages", async () => {
    render(<MockChatContainer />);
    
    // Create new session
    fireEvent.click(screen.getByTestId("new-session"));
    
    // Should have one session
    await waitFor(() => {
      expect(screen.getByTestId("session-list").children.length).toBeGreaterThan(1);
    });
    
    // Send a message
    const input = screen.getByTestId("message-input");
    fireEvent.change(input, { target: { value: "Hello, AI!" } });
    fireEvent.keyDown(input, { key: "Enter" });
    
    // Should display user message
    await waitFor(() => {
      expect(screen.getByText("Hello, AI!")).toBeInTheDocument();
    });
  });

  it("should switch between sessions and maintain separate message histories", async () => {
    render(<MockChatContainer />);
    
    // Create two sessions
    fireEvent.click(screen.getByTestId("new-session"));
    await waitFor(() => expect(screen.getAllByTestId(/^session-session_/).length).toBe(1));
    
    const session1Button = screen.getAllByTestId(/^session-session_/)[0];
    const session1Id = session1Button.getAttribute("data-testid")!.replace("session-", "");
    
    // Send message in session 1
    const input = screen.getByTestId("message-input");
    fireEvent.change(input, { target: { value: "Message in session 1" } });
    fireEvent.keyDown(input, { key: "Enter" });
    
    await waitFor(() => {
      expect(screen.getByText("Message in session 1")).toBeInTheDocument();
    });
    
    // Create second session
    fireEvent.click(screen.getByTestId("new-session"));
    await waitFor(() => expect(screen.getAllByTestId(/^session-session_/).length).toBe(2));
    
    // Session 2 should have no messages from session 1
    expect(screen.queryByText("Message in session 1")).not.toBeInTheDocument();
    
    // Send message in session 2
    fireEvent.change(input, { target: { value: "Message in session 2" } });
    fireEvent.keyDown(input, { key: "Enter" });
    
    await waitFor(() => {
      expect(screen.getByText("Message in session 2")).toBeInTheDocument();
    });
  });

  it("should handle multiple messages in a session", async () => {
    render(<MockChatContainer />);
    
    fireEvent.click(screen.getByTestId("new-session"));
    
    const input = screen.getByTestId("message-input");
    
    // Send multiple messages
    fireEvent.change(input, { target: { value: "First message" } });
    fireEvent.keyDown(input, { key: "Enter" });
    
    await waitFor(() => {
      expect(screen.getByText("First message")).toBeInTheDocument();
    });
    
    fireEvent.change(input, { target: { value: "Second message" } });
    fireEvent.keyDown(input, { key: "Enter" });
    
    await waitFor(() => {
      expect(screen.getByText("Second message")).toBeInTheDocument();
    });
    
    // Both messages should be displayed
    expect(screen.getByText("First message")).toBeInTheDocument();
    expect(screen.getByText("Second message")).toBeInTheDocument();
  });
});
