"use client";

import { useBackendConfigStore } from "@/stores/backendConfigStore";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

export function ServerConfigPanel() {
  const { config, validationErrors, updateServer } = useBackendConfigStore();
  const errors = validationErrors.server;

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Label htmlFor="server-port">端口</Label>
        <Input
          id="server-port"
          type="number"
          min={1}
          max={65535}
          value={config.server.port}
          onChange={(e) => updateServer({ port: parseInt(e.target.value, 10) || 0 })}
          className={errors?.port ? "border-destructive" : ""}
        />
        {errors?.port ? (
          <p className="text-sm text-destructive">{errors.port}</p>
        ) : (
          <p className="text-sm text-muted-foreground">HTTP 服务器端口 (1-65535)</p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="server-host">主机地址</Label>
        <Input
          id="server-host"
          type="text"
          value={config.server.host}
          onChange={(e) => updateServer({ host: e.target.value })}
          className={errors?.host ? "border-destructive" : ""}
        />
        {errors?.host ? (
          <p className="text-sm text-destructive">{errors.host}</p>
        ) : (
          <p className="text-sm text-muted-foreground">服务器绑定的 IP 地址或主机名</p>
        )}
      </div>

      <div className="flex items-center space-x-2">
        <Switch
          id="server-cors"
          checked={config.server.cors}
          onCheckedChange={(checked) => updateServer({ cors: checked })}
        />
        <Label htmlFor="server-cors">启用 CORS</Label>
      </div>
    </div>
  );
}
