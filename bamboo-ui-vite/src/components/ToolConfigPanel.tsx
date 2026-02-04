import React from "react";

export interface ToolConfig {
  endpoint: string;
  apiKey: string;
  model: string;
  environment: "dev" | "staging" | "prod";
  timeoutMs: number;
  retries: number;
  concurrency: number;
  streamResponses: boolean;
  enableCache: boolean;
  enableTracing: boolean;
}

export type SaveStatus = "idle" | "saving" | "saved" | "error";

interface ToolConfigPanelProps {
  config: ToolConfig;
  onChange: (next: Partial<ToolConfig>) => void;
  onReset: () => void;
  saveStatus: SaveStatus;
  lastSavedAt: Date | null;
}

const STATUS_COPY: Record<SaveStatus, string> = {
  idle: "Idle",
  saving: "Saving...",
  saved: "Saved",
  error: "Save failed",
};

const STATUS_STYLES: Record<SaveStatus, string> = {
  idle: "bg-muted text-muted-foreground border-border",
  saving: "bg-blue-500/10 text-blue-700 border-blue-200 dark:bg-blue-500/20 dark:text-blue-300 dark:border-blue-800",
  saved: "bg-emerald-500/10 text-emerald-700 border-emerald-200 dark:bg-emerald-500/20 dark:text-emerald-300 dark:border-emerald-800",
  error: "bg-rose-500/10 text-rose-700 border-rose-200 dark:bg-rose-500/20 dark:text-rose-300 dark:border-rose-800",
};

export default function ToolConfigPanel({
  config,
  onChange,
  onReset,
  saveStatus,
  lastSavedAt,
}: ToolConfigPanelProps) {
  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-border bg-card p-6 shadow-sm">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-lg font-semibold text-foreground">Tool Config</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Changes are saved automatically to local storage.
            </p>
          </div>
          <div className="flex items-center gap-3">
            <span
              className={`rounded-full border px-3 py-1 text-xs font-semibold ${STATUS_STYLES[saveStatus]}`}
            >
              {STATUS_COPY[saveStatus]}
            </span>
            <button
              type="button"
              onClick={onReset}
              className="rounded-full border border-border px-3 py-1 text-xs font-semibold text-foreground hover:bg-accent"
            >
              Reset
            </button>
          </div>
        </div>
        <div className="mt-4 text-xs text-muted-foreground">
          Last saved: {lastSavedAt ? lastSavedAt.toLocaleTimeString() : "--"}
        </div>
      </div>

      <div className="rounded-2xl border border-border bg-card p-6 shadow-sm">
        <h3 className="text-sm font-semibold text-foreground">Connection</h3>
        <div className="mt-4 grid gap-4">
          <label className="grid gap-2 text-sm text-foreground">
            Endpoint URL
            <input
              type="text"
              value={config.endpoint}
              onChange={(event) => onChange({ endpoint: event.target.value })}
              className="h-10 rounded-lg border border-input bg-background px-3 text-sm"
            />
          </label>
          <label className="grid gap-2 text-sm text-foreground">
            API Key
            <input
              type="password"
              value={config.apiKey}
              onChange={(event) => onChange({ apiKey: event.target.value })}
              className="h-10 rounded-lg border border-input bg-background px-3 text-sm"
            />
          </label>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <label className="grid gap-2 text-sm text-foreground">
              Environment
              <select
                value={config.environment}
                onChange={(event) =>
                  onChange({ environment: event.target.value as ToolConfig["environment"] })
                }
                className="h-10 rounded-lg border border-input bg-background px-3 text-sm"
              >
                <option value="dev">Development</option>
                <option value="staging">Staging</option>
                <option value="prod">Production</option>
              </select>
            </label>
            <label className="grid gap-2 text-sm text-foreground">
              Model
              <select
                value={config.model}
                onChange={(event) => onChange({ model: event.target.value })}
                className="h-10 rounded-lg border border-input bg-background px-3 text-sm"
              >
                <option value="bamboo-core">bamboo-core</option>
                <option value="bamboo-plus">bamboo-plus</option>
                <option value="bamboo-lite">bamboo-lite</option>
              </select>
            </label>
          </div>
        </div>
      </div>

      <div className="rounded-2xl border border-border bg-card p-6 shadow-sm">
        <h3 className="text-sm font-semibold text-foreground">Runtime</h3>
        <div className="mt-4 grid gap-4 md:grid-cols-2">
          <label className="grid gap-2 text-sm text-foreground">
            Timeout (ms)
            <input
              type="number"
              min={0}
              value={config.timeoutMs}
              onChange={(event) => onChange({ timeoutMs: Number(event.target.value) || 0 })}
              className="h-10 rounded-lg border border-input bg-background px-3 text-sm"
            />
          </label>
          <label className="grid gap-2 text-sm text-foreground">
            Retries
            <input
              type="number"
              min={0}
              value={config.retries}
              onChange={(event) => onChange({ retries: Number(event.target.value) || 0 })}
              className="h-10 rounded-lg border border-input bg-background px-3 text-sm"
            />
          </label>
          <label className="grid gap-2 text-sm text-foreground">
            Concurrency
            <input
              type="number"
              min={1}
              value={config.concurrency}
              onChange={(event) =>
                onChange({ concurrency: Math.max(1, Number(event.target.value) || 1) })
              }
              className="h-10 rounded-lg border border-input bg-background px-3 text-sm"
            />
          </label>
          <label className="grid gap-2 text-sm text-foreground">
            Streaming
            <select
              value={config.streamResponses ? "on" : "off"}
              onChange={(event) => onChange({ streamResponses: event.target.value === "on" })}
              className="h-10 rounded-lg border border-input bg-background px-3 text-sm"
            >
              <option value="on">Enabled</option>
              <option value="off">Disabled</option>
            </select>
          </label>
        </div>
      </div>

      <div className="rounded-2xl border border-border bg-card p-6 shadow-sm">
        <h3 className="text-sm font-semibold text-foreground">Observability</h3>
        <div className="mt-4 grid gap-4">
          <label className="flex items-center justify-between rounded-lg border border-border px-4 py-3 text-sm text-foreground">
            <div>
              <div className="font-medium">Enable tracing</div>
              <div className="text-xs text-muted-foreground">Trace the full tool call chain.</div>
            </div>
            <input
              type="checkbox"
              checked={config.enableTracing}
              onChange={(event) => onChange({ enableTracing: event.target.checked })}
              className="h-4 w-4"
            />
          </label>
          <label className="flex items-center justify-between rounded-lg border border-border px-4 py-3 text-sm text-foreground">
            <div>
              <div className="font-medium">Enable cache</div>
              <div className="text-xs text-muted-foreground">Reuse tool outputs across requests.</div>
            </div>
            <input
              type="checkbox"
              checked={config.enableCache}
              onChange={(event) => onChange({ enableCache: event.target.checked })}
              className="h-4 w-4"
            />
          </label>
        </div>
      </div>
    </div>
  );
}
