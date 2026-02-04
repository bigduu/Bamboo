"use client";

import React from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";

interface NewSessionButtonProps {
  className?: string;
  onClick?: () => void;
}

export function NewSessionButton({ className, onClick }: NewSessionButtonProps) {
  return (
    <Button
      onClick={onClick}
      className={cn("w-full justify-start gap-2", className)}
      variant="outline"
    >
      <Plus className="h-4 w-4" />
      新建会话
    </Button>
  );
}
