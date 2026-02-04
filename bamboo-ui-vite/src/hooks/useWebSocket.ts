"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  type WebSocketManagerOptions,
  WebSocketManager,
} from "@/lib/websocket";
import type { WebSocketStatus } from "@/types";

export interface UseWebSocketResult {
  status: WebSocketStatus;
  lastMessage: MessageEvent | null;
  error: Event | null;
  connect: () => void;
  disconnect: () => void;
  send: (data: unknown) => void;
}

export const useWebSocket = (
  options: WebSocketManagerOptions | null
): UseWebSocketResult => {
  const managerRef = useRef<WebSocketManager | null>(null);
  const [status, setStatus] = useState<WebSocketStatus>("idle");
  const [lastMessage, setLastMessage] = useState<MessageEvent | null>(null);
  const [error, setError] = useState<Event | null>(null);

  const stableOptions = useMemo(() => {
    if (!options) {
      return null;
    }

    return {
      ...options,
      protocols: options.protocols,
    };
  }, [
    options?.url,
    Array.isArray(options?.protocols)
      ? options?.protocols.join(",")
      : options?.protocols,
    options?.reconnect,
    options?.reconnectAttempts,
    options?.reconnectInterval,
    options?.maxReconnectInterval,
  ]);

  useEffect(() => {
    if (!stableOptions) {
      return;
    }

    const manager = new WebSocketManager(stableOptions);
    managerRef.current = manager;

    const offStatus = manager.on("status", setStatus);
    const offMessage = manager.on("message", (event) => setLastMessage(event));
    const offError = manager.on("error", (event) => setError(event));

    manager.connect();

    return () => {
      offStatus();
      offMessage();
      offError();
      manager.disconnect();
      managerRef.current = null;
    };
  }, [stableOptions]);

  const connect = useCallback(() => {
    managerRef.current?.connect();
  }, []);

  const disconnect = useCallback(() => {
    managerRef.current?.disconnect();
  }, []);

  const send = useCallback((data: unknown) => {
    if (!managerRef.current) {
      throw new Error("WebSocket manager is not initialized.");
    }

    managerRef.current.send(data);
  }, []);

  return {
    status,
    lastMessage,
    error,
    connect,
    disconnect,
    send,
  };
};
