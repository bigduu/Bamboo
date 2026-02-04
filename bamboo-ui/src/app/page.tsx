"use client";

import React, { useEffect, useMemo, useState } from "react";
import ToolCallChain, { ToolCallStep } from "@/components/ToolCallChain";
import ToolConfigPanel, {
  SaveStatus,
  ToolConfig,
} from "@/components/ToolConfigPanel";

const STORAGE_KEY = "bamboo.tool.config.v1";

const DEFAULT_CONFIG: ToolConfig = {
  endpoint: "http://localhost:8000/api/tools",
  apiKey: "",
  model: "bamboo-core",
  environment: "dev",
  timeoutMs: 120000,
  retries: 2,
  concurrency: 4,
  streamResponses: true,
  enableCache: true,
  enableTracing: true,
};

const TOOL_CALL_STEPS: ToolCallStep[] = [
  {
    id: "step-1",
    name: "Plan Router",
    description: "Select the tool call chain based on intent and policy.",
    status: "success",
    startedAt: "09:42:15",
    durationMs: 128,
    input: `{
  "intent": "sync inventory and pricing",
  "priority": "high"
}`,
    output: `{
  "route": ["Inventory.Fetch", "Pricing.Match", "Catalog.Sync"]
}`,
  },
  {
    id: "step-2",
    name: "Inventory.Fetch",
    description: "Pull real-time inventory snapshot from the warehouse API.",
    status: "success",
    startedAt: "09:42:16",
    durationMs: 312,
    input: `{
  "warehouseId": "WH-198",
  "includeBackorder": true
}`,
    output: `{
  "items": 982,
  "backorder": 14,
  "etag": "inv_17c0"
}`,
  },
  {
    id: "step-3",
    name: "Pricing.Match",
    description: "Align pricing rules with the latest inventory payload.",
    status: "running",
    startedAt: "09:42:17",
    durationMs: 840,
    input: `{
  "pricingTier": "enterprise",
  "currency": "USD"
}`,
  },
  {
    id: "step-4",
    name: "Catalog.Sync",
    description: "Publish merged inventory and pricing to the catalog service.",
    status: "pending",
    startedAt: "--",
  },
];

interface StoredConfig {
  version: 1;
  savedAt: string;
  data: ToolConfig;
}

export default function Home() {
  const [config, setConfig] = useState<ToolConfig>(DEFAULT_CONFIG);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");
  const [lastSavedAt, setLastSavedAt] = useState<Date | null>(null);
  const [hydrated, setHydrated] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const stored = JSON.parse(raw) as StoredConfig;
        if (stored?.data) {
          setConfig({ ...DEFAULT_CONFIG, ...stored.data });
          if (stored.savedAt) {
            setLastSavedAt(new Date(stored.savedAt));
          }
          setSaveStatus("saved");
        }
      }
    } catch (error) {
      console.error("Failed to load tool config", error);
    } finally {
      setHydrated(true);
    }
  }, []);

  useEffect(() => {
    if (!hydrated) {
      return;
    }

    setSaveStatus("saving");
    const handle = window.setTimeout(() => {
      try {
        const payload: StoredConfig = {
          version: 1,
          savedAt: new Date().toISOString(),
          data: config,
        };
        window.localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
        setLastSavedAt(new Date(payload.savedAt));
        setSaveStatus("saved");
      } catch (error) {
        console.error("Failed to save tool config", error);
        setSaveStatus("error");
      }
    }, 400);

    return () => {
      window.clearTimeout(handle);
    };
  }, [config, hydrated]);

  const summary = useMemo(() => {
    const total = TOOL_CALL_STEPS.reduce(
      (acc, step) => acc + (step.durationMs ?? 0),
      0
    );
    const running = TOOL_CALL_STEPS.filter((step) => step.status === "running").length;
    const pending = TOOL_CALL_STEPS.filter((step) => step.status === "pending").length;
    const errors = TOOL_CALL_STEPS.filter((step) => step.status === "error").length;

    return {
      total,
      running,
      pending,
      errors,
      steps: TOOL_CALL_STEPS.length,
    };
  }, []);

  const maskedConfig = useMemo(() => {
    const maskedKey = config.apiKey
      ? `${config.apiKey.slice(0, 4)}****${config.apiKey.slice(-2)}`
      : "";
    return {
      ...config,
      apiKey: maskedKey,
    };
  }, [config]);

  return (
    <div className="min-h-screen bg-background">
      <div className="mx-auto max-w-6xl px-6 py-12">
        <header className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
              Tool Orchestration
            </p>
            <h1 className="mt-2 text-3xl font-semibold text-foreground">
              Tool Call Console
            </h1>
            <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
              Manage tool runtime configuration and monitor every hop of your tool call
              chain in real time.
            </p>
          </div>
          <div className="flex items-center gap-3 rounded-full border border-border bg-card px-4 py-2 text-xs font-medium text-muted-foreground">
            <span className="h-2 w-2 rounded-full bg-emerald-500" />
            Auto-save enabled
          </div>
        </header>

        <main className="mt-10 grid gap-8 lg:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
          <section>
            <ToolConfigPanel
              config={config}
              onChange={(next) => setConfig((prev) => ({ ...prev, ...next }))}
              onReset={() => setConfig(DEFAULT_CONFIG)}
              saveStatus={saveStatus}
              lastSavedAt={lastSavedAt}
            />
          </section>

          <section className="space-y-6">
            <div className="rounded-2xl border border-border bg-card p-6 shadow-sm">
              <div className="flex items-start justify-between gap-4">
                <div>
                  <h2 className="text-lg font-semibold text-foreground">Tool Call Chain</h2>
                  <p className="mt-1 text-sm text-muted-foreground">
                    Track the chain of tool calls and inspect payloads for each hop.
                  </p>
                </div>
                <div className="text-right text-xs text-muted-foreground">
                  <div>Trace ID: 7c1a-9f2b</div>
                  <div>Request: inv-sync</div>
                </div>
              </div>
              <div className="mt-4 grid gap-4 text-sm sm:grid-cols-2 lg:grid-cols-5">
                <div className="rounded-xl border border-border bg-muted p-3">
                  <div className="text-xs text-muted-foreground">Steps</div>
                  <div className="mt-1 text-lg font-semibold text-foreground">
                    {summary.steps}
                  </div>
                </div>
                <div className="rounded-xl border border-border bg-muted p-3">
                  <div className="text-xs text-muted-foreground">Total latency</div>
                  <div className="mt-1 text-lg font-semibold text-foreground">
                    {summary.total}ms
                  </div>
                </div>
                <div className="rounded-xl border border-border bg-muted p-3">
                  <div className="text-xs text-muted-foreground">Running</div>
                  <div className="mt-1 text-lg font-semibold text-foreground">
                    {summary.running}
                  </div>
                </div>
                <div className="rounded-xl border border-border bg-muted p-3">
                  <div className="text-xs text-muted-foreground">Pending</div>
                  <div className="mt-1 text-lg font-semibold text-foreground">
                    {summary.pending}
                  </div>
                </div>
                <div className="rounded-xl border border-border bg-muted p-3">
                  <div className="text-xs text-muted-foreground">Errors</div>
                  <div className="mt-1 text-lg font-semibold text-foreground">
                    {summary.errors}
                  </div>
                </div>
              </div>
            </div>

            <ToolCallChain steps={TOOL_CALL_STEPS} />

            <div className="rounded-2xl border border-border bg-card p-6 shadow-sm">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-sm font-semibold text-foreground">Config Preview</h3>
                  <p className="mt-1 text-xs text-muted-foreground">
                    API keys are masked before display.
                  </p>
                </div>
                <span className="text-xs font-medium text-muted-foreground">Live</span>
              </div>
              <pre className="mt-4 whitespace-pre-wrap rounded-xl bg-muted p-4 text-xs text-foreground">
                {JSON.stringify(maskedConfig, null, 2)}
              </pre>
            </div>
          </section>
        </main>
      </div>
    </div>
  );
}
