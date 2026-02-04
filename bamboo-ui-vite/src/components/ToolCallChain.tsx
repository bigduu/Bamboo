import React from "react";

export type ToolCallStatus = "success" | "running" | "error" | "pending";

export interface ToolCallStep {
  id: string;
  name: string;
  description: string;
  status: ToolCallStatus;
  startedAt?: string;
  durationMs?: number;
  input?: string;
  output?: string;
}

const STATUS_STYLES: Record<ToolCallStatus, { label: string; badge: string; dot: string }> = {
  success: {
    label: "Success",
    badge: "bg-emerald-500/10 text-emerald-700 border-emerald-200 dark:bg-emerald-500/20 dark:text-emerald-300 dark:border-emerald-800",
    dot: "bg-emerald-500",
  },
  running: {
    label: "Running",
    badge: "bg-blue-500/10 text-blue-700 border-blue-200 dark:bg-blue-500/20 dark:text-blue-300 dark:border-blue-800",
    dot: "bg-blue-500",
  },
  error: {
    label: "Error",
    badge: "bg-rose-500/10 text-rose-700 border-rose-200 dark:bg-rose-500/20 dark:text-rose-300 dark:border-rose-800",
    dot: "bg-rose-500",
  },
  pending: {
    label: "Pending",
    badge: "bg-muted text-muted-foreground border-border",
    dot: "bg-muted-foreground",
  },
};

interface ToolCallChainProps {
  steps: ToolCallStep[];
}

export default function ToolCallChain({ steps }: ToolCallChainProps) {
  return (
    <div className="space-y-5">
      {steps.map((step, index) => {
        const status = STATUS_STYLES[step.status];
        const isLast = index === steps.length - 1;
        return (
          <div key={step.id} className="flex gap-4">
            <div className="flex flex-col items-center">
              <span
                className={`mt-1 h-3 w-3 rounded-full ${status.dot} ring-4 ring-background`}
              />
              {!isLast && <span className="mt-2 h-full w-px bg-border" />}
            </div>
            <div className="flex-1 rounded-2xl border border-border bg-card p-4 shadow-sm">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="text-base font-semibold text-foreground">{step.name}</h3>
                    <span
                      className={`rounded-full border px-2.5 py-0.5 text-xs font-medium ${status.badge}`}
                    >
                      {status.label}
                    </span>
                  </div>
                  <p className="mt-1 text-sm text-muted-foreground">{step.description}</p>
                </div>
                <div className="text-right text-xs text-muted-foreground">
                  <div>Start: {step.startedAt ?? "--"}</div>
                  <div>Latency: {step.durationMs ? `${step.durationMs}ms` : "--"}</div>
                </div>
              </div>
              {(step.input || step.output) && (
                <details className="mt-3 rounded-xl border border-dashed border-border bg-muted p-3 text-sm text-foreground">
                  <summary className="cursor-pointer text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    Payload
                  </summary>
                  <div className="mt-3 grid gap-3">
                    {step.input && (
                      <div>
                        <div className="text-[11px] font-semibold uppercase text-muted-foreground">
                          Input
                        </div>
                        <pre className="mt-1 whitespace-pre-wrap rounded-lg bg-background p-3 text-xs text-foreground">
                          {step.input}
                        </pre>
                      </div>
                    )}
                    {step.output && (
                      <div>
                        <div className="text-[11px] font-semibold uppercase text-muted-foreground">
                          Output
                        </div>
                        <pre className="mt-1 whitespace-pre-wrap rounded-lg bg-background p-3 text-xs text-foreground">
                          {step.output}
                        </pre>
                      </div>
                    )}
                  </div>
                </details>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
