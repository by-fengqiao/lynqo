import type { QrCodeData } from "@/services/tauri";

export function buildConnectionQrUrl(ip: string, port: number, token: string): string {
  return `http://${ip}:${port}/mobile?token=${encodeURIComponent(token)}`;
}

/**
 * A QR image only needs regeneration when the information embedded in it has
 * changed. Network polling with the same address must leave the rendered SVG
 * alone so a phone can keep scanning it without visual flicker.
 */
export function needsConnectionQrRefresh(
  currentQr: QrCodeData | null,
  ip: string,
  port: number,
  token: string
): boolean {
  return currentQr?.url !== buildConnectionQrUrl(ip, port, token);
}
