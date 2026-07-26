export interface WsMessage {
  type: string;
  payload?: Record<string, unknown>;
}

export type WsConnectionState =
  | "idle"
  | "connecting"
  | "open"
  | "reconnecting"
  | "disconnected";

type Listener = (msg: WsMessage) => void;

export class LanNookWebSocket {
  private ws: WebSocket | null = null;
  private listeners = new Map<string, Set<Listener>>();
  private url = "";
  private reconnectAttempts = 0;
  private readonly maxReconnectAttempts = 10;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private intentionalClose = false;
  private state: WsConnectionState = "idle";

  connect(url: string) {
    const urlChanged = url !== this.url;
    if (
      !urlChanged &&
      (this.ws?.readyState === WebSocket.OPEN ||
        this.ws?.readyState === WebSocket.CONNECTING)
    ) {
      return;
    }

    if (urlChanged) {
      this.closeCurrentSocket();
      this.reconnectAttempts = 0;
    }
    this.url = url;
    this.intentionalClose = false;
    this.transition("connecting");
    this.doConnect();
  }

  private transition(state: WsConnectionState, payload: Record<string, unknown> = {}) {
    this.state = state;
    this.emit("connection.state", {
      type: "connection.state",
      payload: { state, ...payload },
    });
  }

  private doConnect() {
    if (!this.url || this.intentionalClose) return;
    if (
      this.ws?.readyState === WebSocket.OPEN ||
      this.ws?.readyState === WebSocket.CONNECTING
    ) {
      return;
    }

    const socket = new WebSocket(this.url);
    this.ws = socket;
    socket.onopen = () => {
      if (socket !== this.ws) return;
      this.reconnectAttempts = 0;
      this.transition("open");
      this.emit("connected", { type: "connected", payload: {} });
    };
    socket.onmessage = (event) => {
      if (socket !== this.ws) return;
      try {
        const msg = JSON.parse(event.data as string) as WsMessage;
        this.emit(msg.type, msg);
      } catch {
        // Ignore malformed messages from an incompatible endpoint.
      }
    };
    socket.onclose = (event) => {
      if (socket !== this.ws) return;
      this.ws = null;
      if (this.intentionalClose) {
        this.transition("idle");
        return;
      }
      this.emit("disconnected", {
        type: "disconnected",
        payload: { reason: "socket_closed", code: event.code },
      });
      this.scheduleReconnect();
    };
    socket.onerror = () => {
      if (socket === this.ws) socket.close();
    };
  }

  private scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.transition("disconnected", { reason: "reconnect_exhausted" });
      this.emit("reconnect_failed", {
        type: "reconnect_failed",
        payload: { attempts: this.maxReconnectAttempts },
      });
      return;
    }

    const delay = Math.min(1000 * 2 ** this.reconnectAttempts, 30000);
    this.reconnectAttempts += 1;
    this.transition("reconnecting", {
      attempt: this.reconnectAttempts,
      delay,
    });
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.doConnect();
    }, delay);
  }

  private closeCurrentSocket() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    const socket = this.ws;
    this.ws = null;
    if (!socket) return;
    socket.onopen = null;
    socket.onmessage = null;
    socket.onclose = null;
    socket.onerror = null;
    socket.close();
  }

  disconnect() {
    this.intentionalClose = true;
    this.closeCurrentSocket();
    this.transition("idle");
  }

  reconnect() {
    this.closeCurrentSocket();
    this.reconnectAttempts = 0;
    this.intentionalClose = false;
    if (!this.url) return;
    this.transition("connecting");
    this.doConnect();
  }

  getState(): WsConnectionState {
    return this.state;
  }

  on(type: string, fn: Listener) {
    if (!this.listeners.has(type)) this.listeners.set(type, new Set());
    this.listeners.get(type)!.add(fn);
  }

  off(type: string, fn: Listener) {
    this.listeners.get(type)?.delete(fn);
  }

  private emit(type: string, msg: WsMessage) {
    this.listeners.get(type)?.forEach((fn) => fn(msg));
    this.listeners.get("*")?.forEach((fn) => fn(msg));
  }
}

export const wsClient = new LanNookWebSocket();
