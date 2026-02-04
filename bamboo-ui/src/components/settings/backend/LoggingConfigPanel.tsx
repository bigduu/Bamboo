"use client";

import { useBackendConfigStore } from "@/stores/backendConfigStore";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export function LoggingConfigPanel() {
  const { config, validationErrors, updateLogging } = useBackendConfigStore();
  const errors = validationErrors.logging;

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Label htmlFor="logging-level">日志级别</Label>
        <select
          id="logging-level"
          value={config.logging.level}
          onChange={(e) => updateLogging({ level: e.target.value })}
          className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <option value="trace">Trace</option>
          <option value="debug">Debug</option>
          <option value="info">Info</option>
          <option value="warn">Warn</option>
          <option value="error">Error</option>
        </select>
        <p className="text-sm text-muted-foreground">日志记录级别</p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="logging-file">日志文件路径</Label>
        <Input
          id="logging-file"
          type="text"
          value={config.logging.file}
          onChange={(e) => updateLogging({ file: e.target.value })}
          className={errors?.file ? "border-destructive" : ""}
        />
        {errors?.file ? (
          <p className="text-sm text-destructive">{errors.file}</p>
        ) : (
          <p className="text-sm text-muted-foreground">日志文件存储路径</p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="logging-max-size">最大日志大小 (MB)</Label>
        <Input
          id="logging-max-size"
          type="number"
          min={1}
          value={config.logging.max_size_mb}
          onChange={(e) => updateLogging({ max_size_mb: parseInt(e.target.value, 10) || 0 })}
          className={errors?.max_size_mb ? "border-destructive" : ""}
        />
        {errors?.max_size_mb ? (
          <p className="text-sm text-destructive">{errors.max_size_mb}</p>
        ) : (
          <p className="text-sm text-muted-foreground">单个日志文件最大大小（MB）</p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="logging-max-files">最大日志文件数</Label>
        <Input
          id="logging-max-files"
          type="number"
          min={1}
          value={config.logging.max_files}
          onChange={(e) => updateLogging({ max_files: parseInt(e.target.value, 10) || 0 })}
          className={errors?.max_files ? "border-destructive" : ""}
        />
        {errors?.max_files ? (
          <p className="text-sm text-destructive">{errors.max_files}</p>
        ) : (
          <p className="text-sm text-muted-foreground">保留的日志文件数量</p>
        )}
      </div>
    </div>
  );
}
