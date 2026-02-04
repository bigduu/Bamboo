"use client";

import React, { useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import { ScrollArea } from "@/components/ui/scroll-area";
import { MessageBubble } from "./MessageBubble";
import { Loader2 } from "lucide-react";
import type { Message } from "./ChatContainer";

interface MessageListProps {
  messages: Message[];
  isLoading?: boolean;
  className?: string;
}

export function MessageList({
  messages,
  isLoading = false,
  className,
}: MessageListProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  return (
    <ScrollArea
      ref={scrollRef}
      className={cn("flex-1 px-4", className)}
    >
      <div className="space-y-4 py-4">
        {messages.length === 0 ? (
          <div className="flex h-full items-center justify-center text-muted-foreground">
            <p>开始一个新的对话吧</p>
          </div>
        ) : (
          messages.map((message) => (
            <MessageBubble key={message.id} message={message} />
          ))
        )}

        {isLoading && (
          <div className="flex items-center gap-2 py-2 text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            <span className="text-sm">AI 正在思考... </span>
          </div>
        )}
      </div>
    </ScrollArea>
  );
}
