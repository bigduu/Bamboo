"use client";

import React from "react";
import { cn } from "@/lib/utils";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SessionList } from "@/components/session/SessionList";
import { NewSessionButton } from "@/components/session/NewSessionButton";
import { MessageSquare } from "lucide-react";

interface SidebarProps {
  className?: string;
}

export function Sidebar({ className }: SidebarProps) {
  return (
    <aside
      className={cn(
        "flex w-64 flex-col border-r bg-card transition-colors duration-300",
        className
      )}
    >
      <div className="flex h-14 items-center border-b border-border px-4">
        <MessageSquare className="mr-2 h-5 w-5 text-primary" />
        <span className="font-semibold">Bamboo Chat</span>
      </div>

      <div className="p-3">
        <NewSessionButton />
      </div>

      <ScrollArea className="flex-1 px-3">
        <SessionList />
      </ScrollArea>

      <div className="border-t border-border p-3">
        <div className="text-xs text-muted-foreground text-center">
          Bamboo AI Chat v0.1.0
        </div>
      </div>
    </aside>
  );
}
