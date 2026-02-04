import { useState } from "react";
import { useConfigStore } from "@/stores/configStore";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { CheckCircle, XCircle, Loader2, Server, Wifi } from "lucide-react";

export function ServerConfigPanel() {
  const { config, setApiUrl, setWsUrl, testConnection } = useConfigStore();
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    success?: boolean;
    latency?: number;
    error?: string;
  } | null>(null);

  const handleTestConnection = async () => {
    setIsTesting(true);
    setTestResult(null);
    
    const result = await testConnection();
    setTestResult(result);
    setIsTesting(false);
  };

  const isEnvOverride = {
    apiUrl: import.meta.env.VITE_API_URL,
    wsUrl: import.meta.env.VITE_WS_URL,
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Server className="h-5 w-5" />
          Server Configuration
        </CardTitle>
        <CardDescription>
          Configure the backend server addresses. Changes will be saved automatically.
        </CardDescription>
      </CardHeader>
      
      <CardContent className="space-y-6">
        {/* HTTP API URL */}
        <div className="space-y-2">
          <Label htmlFor="api-url" className="flex items-center gap-2">
            HTTP API URL
            {isEnvOverride.apiUrl && (
              <Badge variant="secondary" className="text-xs">
                ENV Override
              </Badge>
            )}
          </Label>
          <Input
            id="api-url"
            type="url"
            placeholder="http://localhost:3000"
            value={config.apiUrl}
            onChange={(e) => setApiUrl(e.target.value)}
            disabled={!!isEnvOverride.apiUrl}
          />
          <p className="text-sm text-muted-foreground">
            The HTTP API endpoint for REST requests
          </p>
        </div>

        {/* WebSocket URL */}
        <div className="space-y-2">
          <Label htmlFor="ws-url" className="flex items-center gap-2">
            <Wifi className="h-4 w-4" />
            WebSocket URL
            {isEnvOverride.wsUrl && (
              <Badge variant="secondary" className="text-xs">
                ENV Override
              </Badge>
            )}
          </Label>
          <Input
            id="ws-url"
            type="url"
            placeholder="ws://localhost:18790"
            value={config.wsUrl}
            onChange={(e) => setWsUrl(e.target.value)}
            disabled={!!isEnvOverride.wsUrl}
          />
          <p className="text-sm text-muted-foreground">
            The WebSocket endpoint for real-time communication
          </p>
        </div>

        {/* Connection Test */}
        <div className="flex items-center gap-4 pt-4 border-t">
          <Button
            onClick={handleTestConnection}
            disabled={isTesting}
            variant="outline"
          >
            {isTesting ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Testing...
              </>
            ) : (
              <>
                <CheckCircle className="mr-2 h-4 w-4" />
                Test Connection
              </>
            )}
          </Button>

          {testResult && (
            <div className="flex items-center gap-2">
              {testResult.success ? (
                <>
                  <CheckCircle className="h-5 w-5 text-emerald-500 dark:text-emerald-400" />
                  <span className="text-sm text-emerald-600 dark:text-emerald-400">
                    Connected ({testResult.latency}ms)
                  </span>
                </>
              ) : (
                <>
                  <XCircle className="h-5 w-5 text-red-500 dark:text-red-400" />
                  <span className="text-sm text-red-600 dark:text-red-400">
                    {testResult.error || "Connection failed"}
                  </span>
                </>
              )}
            </div>
          )}
        </div>

        {/* Environment Variables Info */}
        {(isEnvOverride.apiUrl || isEnvOverride.wsUrl) && (
          <div className="rounded-lg bg-muted p-4 text-sm">
            <p className="font-medium mb-2">Environment Variables Detected</p>
            <ul className="space-y-1 text-muted-foreground">
              {isEnvOverride.apiUrl && (
                <li>VITE_API_URL={import.meta.env.VITE_API_URL}</li>
              )}
              {isEnvOverride.wsUrl && (
                <li>VITE_WS_URL={import.meta.env.VITE_WS_URL}</li>
              )}
            </ul>
            <p className="mt-2 text-xs">
              Environment variables override user settings. To enable manual configuration, unset these variables.
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
