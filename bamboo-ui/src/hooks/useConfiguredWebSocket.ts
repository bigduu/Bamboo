"use client";

import { useEffect, useMemo } from "react";
import { useConfigStore } from "@/stores/configStore";
import { useWebSocket, type UseWebSocketResult } from "./useWebSocket";

export interface UseConfiguredWebSocketOptions {
  enabled?: boolean;
  reconnect?: boolean;
  reconnectAttempts?: number;
  reconnectInterval?: number;
}

export const useConfiguredWebSocket = (
  options: UseConfiguredWebSocketOptions = {}
): UseWebSocketResult => {
  const { wsUrl } = useConfigStore((state) => state.config);
  const { enabled = true, ...wsOptions } = options;

  const wsManagerOptions = useMemo(() => {
    if (!enabled || !wsUrl) {
      return null;
    }

    return {
      url: wsUrl,
      reconnect: true,
      reconnectAttempts: 6,
      reconnectInterval: 1000,
      ...wsOptions,
    };
  }, [wsUrl, enabled, wsOptions]);

  const result = useWebSocket(wsManagerOptions);

  // 当 URL 改变时，自动断开并重新连接
  useEffect(() => {
    if (!enabled || !wsUrl) {
      result.disconnect();
    }
  }, [wsUrl, enabled, result.disconnect]);

  return result;
};
