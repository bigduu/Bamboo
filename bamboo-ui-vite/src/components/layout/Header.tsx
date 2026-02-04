
import React from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Settings, User } from "lucide-react";
import { ThemeToggle } from "@/components/ThemeToggle";

interface HeaderProps {
  className?: string;
}

export function Header({ className }: HeaderProps) {
  return (
    <header
      className={cn(
        "flex h-14 items-center justify-between border-b bg-card px-4 transition-colors duration-300",
        className
      )}
    >
      <div className="flex items-center gap-2">
        <h1 className="text-lg font-semibold">当前会话</h1>
      </div>
      
      <div className="flex items-center gap-2">
        <ThemeToggle className="h-9 w-9" />
        
        <Button variant="ghost" size="icon" className="h-9 w-9">
          <Settings className="h-4 w-4" />
        </Button>
        
        <Button variant="ghost" size="icon" className="h-9 w-9">
          <User className="h-4 w-4" />
        </Button>
      </div>
    </header>
  );
}
