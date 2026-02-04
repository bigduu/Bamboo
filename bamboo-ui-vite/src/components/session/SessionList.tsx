
import React from "react";
import { cn } from "@/lib/utils";
import { SessionItem } from "./SessionItem";

export interface Session {
  id: string;
  title: string;
  lastMessage: string;
  timestamp: Date;
  isActive?: boolean;
}

interface SessionListProps {
  className?: string;
  sessions?: Session[];
  activeSessionId?: string;
  onSelectSession?: (sessionId: string) => void;
}

const mockSessions: Session[] = [
  {
    id: "1",
    title: "新会话 1",
    lastMessage: "你好，请问有什么可以帮助你的？",
    timestamp: new Date(),
    isActive: true,
  },
  {
    id: "2",
    title: "关于 Next.js 的讨论",
    lastMessage: "Next.js 是一个非常好用的 React 框架...",
    timestamp: new Date(Date.now() - 1000 * 60 * 30),
    isActive: false,
  },
  {
    id: "3",
    title: "代码审查",
    lastMessage: "这段代码看起来不错，但是可以优化一下...",
    timestamp: new Date(Date.now() - 1000 * 60 * 60 * 2),
    isActive: false,
  },
];

export function SessionList({
  className,
  sessions = mockSessions,
  activeSessionId,
  onSelectSession,
}: SessionListProps) {
  return (
    <div className={cn("space-y-1 py-2", className)}>
      <div className="px-2 py-1 text-xs font-medium text-muted-foreground">
        会话历史
      </div>
      <div className="space-y-1">
        {sessions.map((session) => (
          <SessionItem
            key={session.id}
            session={session}
            isActive={session.id === activeSessionId || session.isActive}
            onClick={() => onSelectSession?.(session.id)}
          />
        ))}
      </div>
    </div>
  );
}
