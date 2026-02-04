"use client";

import React from "react";
import { AppLayout } from "@/components/layout/AppLayout";
import { MaskingConfigPanel } from "@/components/settings/MaskingConfigPanel";

export default function MaskingSettingsPage() {
  return (
    <AppLayout>
      <div className="max-w-6xl mx-auto">
        <MaskingConfigPanel />
      </div>
    </AppLayout>
  );
}
