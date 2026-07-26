import { describe, expect, it } from "vitest";
import { buildConnectionQrUrl, needsConnectionQrRefresh } from "./connectionQr";

describe("connection QR refresh policy", () => {
  const currentQr = {
    url: "http://192.168.1.10:53317/mobile?token=pairing-token",
    svg: "<svg />",
  };

  it("keeps an unchanged QR image during network polling", () => {
    expect(needsConnectionQrRefresh(currentQr, "192.168.1.10", 53317, "pairing-token")).toBe(false);
  });

  it("rebuilds the QR image when its selected network path changes", () => {
    expect(needsConnectionQrRefresh(currentQr, "192.168.137.1", 53317, "pairing-token")).toBe(true);
  });

  it("encodes a changed pairing token before comparing URLs", () => {
    expect(buildConnectionQrUrl("192.168.1.10", 53317, "a token")).toContain("a%20token");
    expect(needsConnectionQrRefresh(currentQr, "192.168.1.10", 53317, "new-token")).toBe(true);
  });
});
