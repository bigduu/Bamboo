
import { useBackendConfigStore } from "@/stores/backendConfigStore";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export function StorageConfigPanel() {
  const { config, validationErrors, updateStorage } = useBackendConfigStore();
  const errors = validationErrors.storage;

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Label htmlFor="storage-type">存储类型</Label>
        <select
          id="storage-type"
          value={config.storage.type}
          onChange={(e) => updateStorage({ type: e.target.value })}
          className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <option value="jsonl">JSONL</option>
          <option value="sqlite">SQLite</option>
          <option value="memory">Memory</option>
        </select>
        <p className="text-sm text-muted-foreground">会话存储类型</p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="storage-path">存储路径</Label>
        <Input
          id="storage-path"
          type="text"
          value={config.storage.path}
          onChange={(e) => updateStorage({ path: e.target.value })}
          className={errors?.path ? "border-destructive" : ""}
        />
        {errors?.path ? (
          <p className="text-sm text-destructive">{errors.path}</p>
        ) : (
          <p className="text-sm text-muted-foreground">会话存储文件路径或目录</p>
        )}
      </div>
    </div>
  );
}
