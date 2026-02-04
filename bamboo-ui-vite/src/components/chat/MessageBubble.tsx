
import React from "react";
import { cn } from "@/lib/utils";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Card, CardContent } from "@/components/ui/card";
import { User, Bot } from "lucide-react";
import type { Message } from "./ChatContainer";

interface MessageBubbleProps {
  message: Message;
}

export function MessageBubble({ message }: MessageBubbleProps) {
  const isUser = message.role === "user";

  return (
    <div
      className={cn(
        "flex w-full gap-3 py-2",
        isUser ? "flex-row-reverse" : "flex-row"
      )}
    >
      <Avatar className={cn("h-8 w-8", isUser ? "bg-primary" : "bg-muted")}>
        <AvatarFallback>
          {isUser ? (
            <User className="h-4 w-4" />
          ) : (
            <Bot className="h-4 w-4" />
          )}
        </AvatarFallback>
      </Avatar>

      <div
        className={cn(
          "flex max-w-[80%] flex-col gap-1",
          isUser ? "items-end" : "items-start"
        )}
      >
        <Card
          className={cn(
            "border-0",
            isUser
              ? "bg-primary text-primary-foreground"
              : "bg-muted text-muted-foreground"
          )}
        >
          <CardContent className="p-3">
            <div className="prose prose-sm max-w-none dark:prose-invert">
              {message.content.split("\n").map((line, i) => (
                <React.Fragment key={i}>
                  {line.startsWith("```") ? (
                    <pre className="mt-2 rounded bg-black/10 dark:bg-white/10 p-2 text-xs overflow-x-auto">
                      <code className="text-foreground">{line.replace(/```/g, "")}</code>
                    </pre>
                  ) : line.startsWith("`") && line.endsWith("`") ? (
                    <code className="rounded bg-black/10 dark:bg-white/10 px-1 text-xs text-foreground">
                      {line.replace(/`/g, "")}
                    </code>
                  ) : (
                    <p className="m-0 leading-relaxed">{line || "\u00a0"}</p>
                  )}
                </React.Fragment>
              ))}
            </div>
          </CardContent>
        </Card>

        <span className="text-xs text-muted-foreground">
          {message.timestamp.toLocaleTimeString("zh-CN", {
            hour: "2-digit",
            minute: "2-digit",
          })}
        </span>
      </div>
    </div>
  );
}
