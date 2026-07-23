export interface WsMessage {
  type: string;
  payload?: Record<string, unknown>;
}

type Listener = (msg: WsMessage) => void;

export class LynqoWebSocket {
  private ws: WebSocket | null = null;
  private listeners = new Map<string, Set<Listener>>();
  private url = "";
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private intentionalClose = false;

  connect(url: string) {
    this.url = url;
    this.intentionalClose = false;
    this.doConnect();
  }

  private doConnect() {
    if (this.ws?.readyState === WebSocket.OPEN) return;
    this.ws = new WebSocket(this.url);
    this.ws.onopen = () => {
      this.reconnectAttempts = 0;
      this.emit("connected", { type: "connected", payload: {} });
    };
    this.ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string);
        this.emit(msg.type, msg);
      } catch {
        // Ignore malformed messages
      }
    };
    this.ws.onclose = () => {
      if (!this.intentionalClose) this.scheduleReconnect();
    };
    this.ws.onerror = () => {
      this.ws?.close();
    };
  }

  private scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      // Give up loudly so the UI can surface the failure (BUG #11)
      this.emit("disconnected", {
        type: "disconnected",
        payload: { reason: "reconnect_exhausted" },
      });
      this.emit("reconnect_failed", {
        type: "reconnect_failed",
        payload: { attempts: this.maxReconnectAttempts },
      });
      return;
    }
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
    this.reconnectAttempts++;
    this.reconnectTimer = setTimeout(() => this.doConnect(), delay);
  }

  disconnect() {
    this.intentionalClose = true;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.ws?.close();
    this.ws = null;
  }

  /**
   * Manual reconnect (e.g. after reconnect attempts were exhausted).
   * Resets the attempt counter and opens a fresh connection.
   */
  reconnect() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.reconnectAttempts = 0;
    this.intentionalClose = false;
    if (this.ws) {
      // Detach handlers so the stale onclose cannot trigger a ghost reconnect
      this.ws.onopen = null;
      this.ws.onmessage = null;
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.close();
      this.ws = null;
    }
    if (this.url) this.doConnect();
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

export const wsClient = new LynqoWebSocket();
