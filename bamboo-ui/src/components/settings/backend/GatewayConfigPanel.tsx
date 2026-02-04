"use client";

import { useBackendConfigStore } from "@/stores/backendConfigStore";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

export function GatewayConfigPanel() {
  const { config, validationErrors, updateGateway } = useBackendConfigStore();
  const errors = validationErrors.gateway;

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-2">
        <Switch
          id="gateway-enabled"
          checked={config.gateway.enabled}
          onCheckedChange={(checked) => updateGateway({ enabled: checked })}
        />
        <Label htmlFor="gateway-enabled">启用网关</Label>
      </div>

      <div className="space-y-2">
        <Label htmlFor="gateway-bind">绑定地址</Label>
        <Input
          id="gateway-bind"
          type="text"
          value={config.gateway.bind}
          onChange={(e) => updateGateway({ bind: e.target.value })}
          className={errors?.bind ? "border-destructive" : ""}
          disabled={!config.gateway.enabled}
        />
        {errors?.bind ? (
          <p className="text-sm text-destructive">{errors.bind}</p>
        ) : (
          <p className="text-sm text-muted-foreground">WebSocket 网关绑定地址 (格式: host:port)</p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="gateway-auth-token">认证令牌 (可选)</Label>
        <Input
          id="gateway-auth-token"
          type="password"
          value={config.gateway.auth_token || ""}
          onChange={(e) => updateGateway({ auth_token: e.target.value || null })}
          placeholder="留空表示不启用认证"
          disabled={!config.gateway.enabled}
        />
        <p className="text-sm text-muted-foreground">WebSocket 连接认证令牌，留空表示不启用认证</p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="gateway-max-connections">最大连接数</Label>
        <Input
          id="gateway-max-connections"
          type="number"
          min={1}
          value={config.gateway.max_connections}
          onChange={(e) => updateGateway({ max_connections: parseInt(e.target.value, 10) || 0 })}
          className={errors?.max_connections ? "border-destructive" : ""}
          disabled={!config.gateway.enabled}
        />
        {errors?.max_connections ? (
          <p className="text-sm text-destructive">{errors.max_connections}</p>
        ) : (
          <p className="text-sm text-muted-foreground">最大 WebSocket 连接数</p>
        )}
      </div>

      <div className="space-y-2">
        <Label htmlFor="gateway-heartbeat">心跳间隔 (秒)</Label>
        <Input
          id="gateway-heartbeat"
          type="number"
          min={1}
          value={config.gateway.heartbeat_interval_secs}
          onChange={(e) => updateGateway({ heartbeat_interval_secs: parseInt(e.target.value, 10) || 0 })}
          className={errors?.heartbeat_interval_secs ? "border-destructive" : ""}
          disabled={!config.gateway.enabled}
        />
        {errors?.heartbeat_interval_secs ? (
          <p className="text-sm text-destructive">{errors.heartbeat_interval_secs}</p>
        ) : (
          <p className="text-sm text-muted-foreground">WebSocket 心跳检测间隔（秒）</p>
        )}
      </div>
    </div>
  );
}
