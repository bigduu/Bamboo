import type { WebSocketStatus } from "@/types";

export type WebSocketEventName = "open" | "message" | "close" | "error" | "status";

export interface WebSocketManagerOptions {
  url: string;
  protocols?: string | string[];
  reconnect?: boolean;
  reconnectAttempts?: number;
  reconnectInterval?: number;
  maxReconnectInterval?: number;
}

export type WebSocketListenerMap = {
  open: (event: Event) => void;
  message: (event: MessageEvent) => void;
  close: (event: CloseEvent) => void;
  error: (event: Event) => void;
  status: (status: WebSocketStatus) => void;
};

export class WebSocketManager {
  private options: Required<WebSocketManagerOptions>;
  private socket: WebSocket | null = null;
  private status: WebSocketStatus = "idle";
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectCount = 0;
  private manuallyClosed = false;
  private listeners: {
    [K in WebSocketEventName]: Set<WebSocketListenerMap[K]>;
  } = {
    open: new Set(),
    message: new Set(),
    close: new Set(),
    error: new Set(),
    status: new Set(),
  };

  constructor(options: WebSocketManagerOptions) {
    this.options = {
      url: options.url,
      protocols: options.protocols ?? [],
      reconnect: options.reconnect ?? true,
      reconnectAttempts: options.reconnectAttempts ?? 6,
      reconnectInterval: options.reconnectInterval ?? 1000,
      maxReconnectInterval: options.maxReconnectInterval ?? 30_000,
    };
  }

  connect() {
    if (typeof WebSocket === "undefined") {
      throw new Error("WebSocket is not available in this environment.");
    }

    if (this.status === "open" || this.status === "connecting") {
      return;
    }

    this.manuallyClosed = false;
    this.setStatus("connecting");

    try {
      this.socket = new WebSocket(this.options.url, this.options.protocols);
    } catch (error) {
      this.handleError(error instanceof Error ? error : undefined);
      this.scheduleReconnect();
      return;
    }

    this.socket.onopen = (event) => {
      this.reconnectCount = 0;
      this.setStatus("open");
      this.emit("open", event);
    };

    this.socket.onmessage = (event) => {
      this.emit("message", event);
    };

    this.socket.onerror = (event) => {
      this.setStatus("error");
      this.emit("error", event);
    };

    this.socket.onclose = (event) => {
      this.emit("close", event);
      this.socket = null;
      if (!this.manuallyClosed) {
        this.setStatus("closed");
        this.scheduleReconnect();
      }
    };
  }

  disconnect(code?: number, reason?: string) {
    this.manuallyClosed = true;
    this.clearReconnect();

    if (this.socket) {
      this.socket.close(code, reason);
      this.socket = null;
    }

    this.setStatus("closed");
  }

  send(data: unknown) {
    if (!this.socket || this.status !== "open") {
      throw new Error("WebSocket is not connected.");
    }

    const payload = typeof data === "string" ? data : JSON.stringify(data);
    this.socket.send(payload);
  }

  getStatus() {
    return this.status;
  }

  on<T extends WebSocketEventName>(
    event: T,
    listener: WebSocketListenerMap[T]
  ) {
    this.listeners[event].add(listener);
    return () => this.off(event, listener);
  }

  off<T extends WebSocketEventName>(
    event: T,
    listener: WebSocketListenerMap[T]
  ) {
    this.listeners[event].delete(listener);
  }

  private emit<T extends WebSocketEventName>(
    event: T,
    payload: Parameters<WebSocketListenerMap[T]>[0]
  ) {
    this.listeners[event].forEach((listener) => listener(payload as never));
  }

  private setStatus(nextStatus: WebSocketStatus) {
    if (this.status === nextStatus) {
      return;
    }

    this.status = nextStatus;
    this.emit("status", nextStatus);
  }

  private scheduleReconnect() {
    if (!this.options.reconnect) {
      return;
    }

    if (this.reconnectCount >= this.options.reconnectAttempts) {
      return;
    }

    this.clearReconnect();

    const delay = Math.min(
      this.options.reconnectInterval * 2 ** this.reconnectCount,
      this.options.maxReconnectInterval
    );

    this.reconnectCount += 1;

    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, delay);
  }

  private clearReconnect() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private handleError(error?: Error) {
    this.setStatus("error");
    if (error) {
      console.error(error);
    }
  }
}
