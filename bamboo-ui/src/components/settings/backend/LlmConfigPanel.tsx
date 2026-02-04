"use client";

import { useState } from "react";
import { useBackendConfigStore } from "@/stores/backendConfigStore";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Plus, Trash2 } from "lucide-react";

export function LlmConfigPanel() {
  const {
    config,
    validationErrors,
    updateDefaultProvider,
    addProvider,
    updateProvider,
    removeProvider,
  } = useBackendConfigStore();
  
  const [newProviderKey, setNewProviderKey] = useState("");
  const errors = validationErrors.llm;
  const providers = config.llm.providers;

  const handleAddProvider = () => {
    if (newProviderKey.trim()) {
      addProvider(newProviderKey.trim());
      setNewProviderKey("");
    }
  };

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Label>默认 Provider</Label>
        <select
          value={config.llm.default_provider}
          onChange={(e) => updateDefaultProvider(e.target.value)}
          className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {Object.keys(providers).map((key) => (
            <option key={key} value={key}>
              {key}
            </option>
          ))}
        </select>
        {errors?.default_provider && (
          <p className="text-sm text-destructive">{errors.default_provider}</p>
        )}
      </div>

      <div className="flex items-center gap-2">
        <Input
          placeholder="新 Provider 名称"
          value={newProviderKey}
          onChange={(e) => setNewProviderKey(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleAddProvider()}
        />
        <Button onClick={handleAddProvider} size="sm">
          <Plus className="h-4 w-4 mr-1" />
          添加
        </Button>
      </div>

      <div className="space-y-4">
        {Object.entries(providers).map(([key, provider]) => (
          <Card key={key}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <div className="flex items-center gap-2">
                <CardTitle className="text-lg">{key}</CardTitle>
                {config.llm.default_provider === key && (
                  <Badge variant="secondary">默认</Badge>
                )}
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => removeProvider(key)}
                disabled={Object.keys(providers).length <= 1}
              >
                <Trash2 className="h-4 w-4 text-destructive" />
              </Button>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center space-x-2">
                <Switch
                  id={`${key}-enabled`}
                  checked={provider.enabled}
                  onCheckedChange={(checked) =>
                    updateProvider(key, { enabled: checked })
                  }
                />
                <Label htmlFor={`${key}-enabled`}>启用</Label>
              </div>

              <div className="space-y-2">
                <Label>Base URL</Label>
                <Input
                  value={provider.base_url}
                  onChange={(e) => updateProvider(key, { base_url: e.target.value })}
                  className={errors?.providers?.[key]?.base_url ? "border-destructive" : ""}
                />
                {errors?.providers?.[key]?.base_url && (
                  <p className="text-sm text-destructive">{errors.providers[key].base_url}</p>
                )}
              </div>

              <div className="space-y-2">
                <Label>模型</Label>
                <Input
                  value={provider.model}
                  onChange={(e) => updateProvider(key, { model: e.target.value })}
                  className={errors?.providers?.[key]?.model ? "border-destructive" : ""}
                />
                {errors?.providers?.[key]?.model && (
                  <p className="text-sm text-destructive">{errors.providers[key].model}</p>
                )}
              </div>

              <div className="space-y-2">
                <Label>认证类型</Label>
                <select
                  value={provider.auth_type}
                  onChange={(e) => updateProvider(key, { auth_type: e.target.value })}
                  className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <option value="bearer">Bearer Token</option>
                  <option value="api_key">API Key</option>
                  <option value="none">无认证</option>
                </select>
              </div>

              <div className="space-y-2">
                <Label>环境变量名</Label>
                <Input
                  value={provider.env}
                  onChange={(e) => updateProvider(key, { env: e.target.value })}
                  className={errors?.providers?.[key]?.env ? "border-destructive" : ""}
                />
                {errors?.providers?.[key]?.env && (
                  <p className="text-sm text-destructive">{errors.providers[key].env}</p>
                )}
                <p className="text-sm text-muted-foreground">存储 API Key 的环境变量名称</p>
              </div>

              <div className="space-y-2">
                <Label>超时 (秒)</Label>
                <Input
                  type="number"
                  min={1}
                  value={provider.timeout_seconds}
                  onChange={(e) =>
                    updateProvider(key, { timeout_seconds: parseInt(e.target.value, 10) || 0 })
                  }
                  className={errors?.providers?.[key]?.timeout_seconds ? "border-destructive" : ""}
                />
                {errors?.providers?.[key]?.timeout_seconds && (
                  <p className="text-sm text-destructive">{errors.providers[key].timeout_seconds}</p>
                )}
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
