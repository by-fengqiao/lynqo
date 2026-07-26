/**
 * Reads a current browser-storage key and imports the first available legacy
 * value when a product rename changes the key namespace. Legacy keys are kept
 * so an older cached client can still read its own state during rollout.
 */
export function readAndMigrateLocalStorageValue(
  currentKey: string,
  legacyKeys: readonly string[]
): string | null {
  if (typeof window === "undefined") return null;

  try {
    const currentValue = window.localStorage.getItem(currentKey);
    if (currentValue !== null) return currentValue;

    for (const legacyKey of legacyKeys) {
      const legacyValue = window.localStorage.getItem(legacyKey);
      if (legacyValue === null) continue;
      window.localStorage.setItem(currentKey, legacyValue);
      return legacyValue;
    }
  } catch {
    // Restricted WebViews and private browsing can disable storage. Callers
    // fall back to their normal in-memory behavior.
  }

  return null;
}
