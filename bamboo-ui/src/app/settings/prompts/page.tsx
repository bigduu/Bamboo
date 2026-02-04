"use client";

import React from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { PromptConfigPanel } from "@/components/prompts/PromptConfigPanel";

export default function PromptsSettingsPage() {
  return (
    <AppLayout>
      <div className="max-w-6xl mx-auto">
        <PromptConfigPanel />
      </div>
    </AppLayout>
  );
}
