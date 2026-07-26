import { shallowRef } from "vue";
import { readAndMigrateLocalStorageValue } from "@/utils/storage";

export type LegalConsentStatus = "pending" | "accepted" | "declined";

export const LEGAL_CONSENT_VERSION = "2026-07-23";
const LEGAL_CONSENT_STORAGE_KEY = "lannook.legal-consent-version";
const LEGACY_LEGAL_CONSENT_STORAGE_KEYS = ["lynqo.legal-consent-version"] as const;

function hasAcceptedCurrentVersion(): boolean {
  try {
    return (
      readAndMigrateLocalStorageValue(
        LEGAL_CONSENT_STORAGE_KEY,
        LEGACY_LEGAL_CONSENT_STORAGE_KEYS
      ) === LEGAL_CONSENT_VERSION
    );
  } catch {
    // Storage can be unavailable in a restricted WebView. Keep the notice visible.
    return false;
  }
}

/**
 * Keeps one local, versioned acknowledgement for the installed desktop app.
 * It is never sent to the LAN service or any third party.
 */
export function useLegalConsent() {
  const status = shallowRef<LegalConsentStatus>(
    hasAcceptedCurrentVersion() ? "accepted" : "pending"
  );

  function accept() {
    try {
      window.localStorage.setItem(LEGAL_CONSENT_STORAGE_KEY, LEGAL_CONSENT_VERSION);
    } catch (error) {
      console.warn("[legal] Unable to persist acknowledgement:", error);
    }
    status.value = "accepted";
  }

  function decline() {
    // Do not persist a rejection. The next app launch must ask again.
    status.value = "declined";
  }

  function reconsider() {
    status.value = "pending";
  }

  return { status, accept, decline, reconsider };
}
