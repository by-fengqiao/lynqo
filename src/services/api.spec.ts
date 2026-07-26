import { describe, expect, it } from "vitest";
import { getApiErrorMessage } from "./api";

describe("getApiErrorMessage", () => {
  it("reads the structured backend error contract", () => {
    expect(
      getApiErrorMessage(
        { error: { code: "TRANSFER_COMPLETED", message: "Transfer already completed" } },
        409
      )
    ).toBe("Transfer already completed");
  });

  it("keeps compatibility with an older string error response", () => {
    expect(getApiErrorMessage({ error: "Device not approved" }, 403)).toBe(
      "Device not approved"
    );
  });

  it("falls back to the HTTP status for malformed responses", () => {
    expect(getApiErrorMessage("not-json", 502)).toBe("HTTP 502");
  });
});
