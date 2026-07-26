import { afterEach, describe, expect, it, vi } from "vitest";
import { readAndMigrateLocalStorageValue } from "./storage";

function createStorage(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem: vi.fn((key: string) => values.get(key) ?? null),
    setItem: vi.fn((key: string, value: string) => values.set(key, value)),
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("readAndMigrateLocalStorageValue", () => {
  it("prefers an existing LanNook value", () => {
    const storage = createStorage({ "lannook.locale": "en-US", "lynqo.locale": "zh-CN" });
    vi.stubGlobal("window", { localStorage: storage });

    expect(readAndMigrateLocalStorageValue("lannook.locale", ["lynqo.locale"])).toBe("en-US");
    expect(storage.setItem).not.toHaveBeenCalled();
  });

  it("copies a LYNQO value into the LanNook key", () => {
    const storage = createStorage({ "lynqo.locale": "zh-CN" });
    vi.stubGlobal("window", { localStorage: storage });

    expect(readAndMigrateLocalStorageValue("lannook.locale", ["lynqo.locale"])).toBe("zh-CN");
    expect(storage.setItem).toHaveBeenCalledWith("lannook.locale", "zh-CN");
  });

  it("returns null when browser storage is unavailable", () => {
    vi.stubGlobal("window", { localStorage: { getItem: () => { throw new Error("blocked"); } } });

    expect(readAndMigrateLocalStorageValue("lannook.locale", ["lynqo.locale"])).toBeNull();
  });
});
