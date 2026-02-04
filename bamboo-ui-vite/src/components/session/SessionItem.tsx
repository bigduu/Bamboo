
import React from "react";
import { cn } from "@/lib/utils";
import { MessageSquare, MoreHorizontal, Trash2 } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import type { Session } from "./SessionList";

interface SessionItemProps {
  session: Session;
  isActive?: boolean;
  onClick?: () => void;
  onDelete?: () => void;
}

export function SessionItem({
  session,
  isActive = false,
  onClick,
  onDelete,
}: SessionItemProps) {
  const formatTime = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / (1000 * 60));
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (minutes < 1) return "刚刚";
    if (minutes < 60) return `${minutes}分钟前`;
    if (hours < 24) return `${hours}小时前`;
    return `${days}天前`;
  };

  return (
    <div
      onClick={onClick}
      className={cn(
        "group flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 transition-colors",
        isActive
          ? "bg-primary/10 text-primary"
          : "hover:bg-accent hover:text-accent-foreground"
      )}
    >
      <MessageSquare className="h-4 w-4 shrink-0" />
      
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between">
          <span className="truncate text-sm font-medium">{session.title}</span>
          <span className="text-xs text-muted-foreground ml-2">
            {formatTime(session.timestamp)}
          </span>
        </div>
        <p className="truncate text-xs text-muted-foreground">
          {session.lastMessage}
        </p>
      </div>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 opacity-0 group-hover:opacity-100"
            onClick={(e) => e.stopPropagation()}
          >
            <MoreHorizontal className="h-3 w-3" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem
            onClick={(e) => {
              e.stopPropagation();
              onDelete?.();
            }}
            className="text-destructive"
          >
            <Trash2 className="mr-2 h-4 w-4" />
            删除会话
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
