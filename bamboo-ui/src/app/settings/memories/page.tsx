"use client";

import React from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { MemoryConfigPanel } from "@/components/memories/MemoryConfigPanel";

export default function MemoriesSettingsPage() {
  return (
    <AppLayout>
      <div className="max-w-6xl mx-auto">
        <MemoryConfigPanel />
      </div>
    </AppLayout>
  );
}
