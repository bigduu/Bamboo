"use client";

import { useEffect } from "react";
import { useBackendConfigStore } from "@/stores/backendConfigStore";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Loader2, Save, RotateCcw, Server, Shield, Brain, Wrench, Bot, Database, FileText } from "lucide-react";
import { ServerConfigPanel } from "@/components/settings/backend/ServerConfigPanel";
import { GatewayConfigPanel } from "@/components/settings/backend/GatewayConfigPanel";
import { LlmConfigPanel } from "@/components/settings/backend/LlmConfigPanel";
import { SkillsConfigPanel } from "@/components/settings/backend/SkillsConfigPanel";
import { AgentConfigPanel } from "@/components/settings/backend/AgentConfigPanel";
import { StorageConfigPanel } from "@/components/settings/backend/StorageConfigPanel";
import { LoggingConfigPanel } from "@/components/settings/backend/LoggingConfigPanel";

export default function BackendSettingsPage() {
  const {
    loading,
    saving,
    error,
    isValid,
    hasLoaded,
    lastLoadedAt,
    lastSavedAt,
    loadConfig,
    saveConfig,
    resetConfig,
  } = useBackendConfigStore();

  useEffect(() => {
    if (!hasLoaded) {
      loadConfig();
    }
  }, [hasLoaded, loadConfig]);

  const handleSave = async () => {
    const result = await saveConfig();
    if (result.success) {
      alert("配置已保存");
    } else {
      alert(`保存失败: ${result.error?.message || "未知错误"}`);
    }
  };

  const handleReset = () => {
    if (confirm("确定要重置所有配置到默认值吗？")) {
      resetConfig();
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin" />
        <span className="ml-2">加载配置中...</span>
      </div>
    );
  }

  return (
    <div className="container max-w-6xl py-8">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-3xl font-bold">后端配置</h1>
          <p className="text-muted-foreground mt-1">
            管理 Bamboo 后端服务的所有配置项
          </p>
        </div>
        <div className="flex items-center gap-4">
          {!isValid && (
            <Badge variant="destructive">配置有误</Badge>
          )}
          {lastSavedAt && (
            <span className="text-xs text-muted-foreground">
              上次保存: {new Date(lastSavedAt).toLocaleString()}
            </span>
          )}
          <Button variant="outline" onClick={handleReset}>
            <RotateCcw className="mr-2 h-4 w-4" />
            重置
          </Button>
          <Button onClick={handleSave} disabled={saving || !isValid}>
            {saving ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                保存中...
              </>
            ) : (
              <>
                <Save className="mr-2 h-4 w-4" />
                保存配置
              </>
            )}
          </Button>
        </div>
      </div>

      {error && (
        <Alert variant="destructive" className="mb-6">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <Tabs defaultValue="server" className="space-y-6">
        <TabsList className="grid grid-cols-7 w-full">
          <TabsTrigger value="server" className="flex items-center gap-2">
            <Server className="h-4 w-4" />
            服务器
          </TabsTrigger>
          <TabsTrigger value="gateway" className="flex items-center gap-2">
            <Shield className="h-4 w-4" />
            网关
          </TabsTrigger>
          <TabsTrigger value="llm" className="flex items-center gap-2">
            <Brain className="h-4 w-4" />
            LLM
          </TabsTrigger>
          <TabsTrigger value="skills" className="flex items-center gap-2">
            <Wrench className="h-4 w-4" />
            Skills
          </TabsTrigger>
          <TabsTrigger value="agent" className="flex items-center gap-2">
            <Bot className="h-4 w-4" />
            Agent
          </TabsTrigger>
          <TabsTrigger value="storage" className="flex items-center gap-2">
            <Database className="h-4 w-4" />
            存储
          </TabsTrigger>
          <TabsTrigger value="logging" className="flex items-center gap-2">
            <FileText className="h-4 w-4" />
            日志
          </TabsTrigger>
        </TabsList>

        <TabsContent value="server">
          <Card>
            <CardHeader>
              <CardTitle>服务器配置</CardTitle>
              <CardDescription>配置 HTTP 服务器的端口、主机和 CORS 设置</CardDescription>
            </CardHeader>
            <CardContent>
              <ServerConfigPanel />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="gateway">
          <Card>
            <CardHeader>
              <CardTitle>网关配置</CardTitle>
              <CardDescription>配置 WebSocket 网关的绑定地址、认证和连接设置</CardDescription>
            </CardHeader>
            <CardContent>
              <GatewayConfigPanel />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="llm">
          <Card>
            <CardHeader>
              <CardTitle>LLM 配置</CardTitle>
              <CardDescription>配置大语言模型提供商和默认模型设置</CardDescription>
            </CardHeader>
            <CardContent>
              <LlmConfigPanel />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="skills">
          <Card>
            <CardHeader>
              <CardTitle>Skills 配置</CardTitle>
              <CardDescription>配置 Skills 目录和自动重载设置</CardDescription>
            </CardHeader>
            <CardContent>
              <SkillsConfigPanel />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="agent">
          <Card>
            <CardHeader>
              <CardTitle>Agent 配置</CardTitle>
              <CardDescription>配置 Agent 的最大轮次、超时和系统提示词</CardDescription>
            </CardHeader>
            <CardContent>
              <AgentConfigPanel />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="storage">
          <Card>
            <CardHeader>
              <CardTitle>存储配置</CardTitle>
              <CardDescription>配置会话存储类型和路径</CardDescription>
            </CardHeader>
            <CardContent>
              <StorageConfigPanel />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="logging">
          <Card>
            <CardHeader>
              <CardTitle>日志配置</CardTitle>
              <CardDescription>配置日志级别、文件路径和轮转设置</CardDescription>
            </CardHeader>
            <CardContent>
              <LoggingConfigPanel />
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
