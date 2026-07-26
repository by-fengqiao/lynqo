import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LanNookWebSocket, type WsMessage } from "./websocket";

class FakeWebSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  static instances: FakeWebSocket[] = [];

  readonly url: string;
  readyState = FakeWebSocket.CONNECTING;
  onopen: (() => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }

  open() {
    this.readyState = FakeWebSocket.OPEN;
    this.onopen?.();
  }

  close(code = 1000) {
    if (this.readyState === FakeWebSocket.CLOSED) return;
    this.readyState = FakeWebSocket.CLOSED;
    this.onclose?.({ code } as CloseEvent);
  }
}

describe("LanNookWebSocket", () => {
  beforeEach(() => {
    FakeWebSocket.instances = [];
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", FakeWebSocket);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("replaces an open socket when its token URL changes", () => {
    const client = new LanNookWebSocket();
    client.connect("ws://host/ws?token=first");
    FakeWebSocket.instances[0].open();

    client.connect("ws://host/ws?token=second");

    expect(FakeWebSocket.instances).toHaveLength(2);
    expect(FakeWebSocket.instances[0].readyState).toBe(FakeWebSocket.CLOSED);
    expect(FakeWebSocket.instances[1].url).toContain("token=second");
    client.disconnect();
  });

  it("announces reconnecting as soon as a live socket closes", () => {
    const client = new LanNookWebSocket();
    const states: string[] = [];
    client.on("connection.state", (message: WsMessage) => {
      if (typeof message.payload?.state === "string") states.push(message.payload.state);
    });
    client.connect("ws://host/ws?token=session");
    FakeWebSocket.instances[0].open();
    FakeWebSocket.instances[0].close(1006);

    expect(states).toContain("open");
    expect(states[states.length - 1]).toBe("reconnecting");
    client.disconnect();
  });

  it("does not create a ghost reconnect after an intentional close", () => {
    const client = new LanNookWebSocket();
    client.connect("ws://host/ws?token=session");
    FakeWebSocket.instances[0].open();
    client.disconnect();
    vi.advanceTimersByTime(60_000);

    expect(FakeWebSocket.instances).toHaveLength(1);
    expect(client.getState()).toBe("idle");
  });
});
