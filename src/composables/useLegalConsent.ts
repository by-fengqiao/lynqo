import { shallowRef } from "vue";

export type LegalConsentStatus = "pending" | "accepted" | "declined";

export const LEGAL_CONSENT_VERSION = "2026-07-23";
const LEGAL_CONSENT_STORAGE_KEY = "lynqo.legal-consent-version";

function hasAcceptedCurrentVersion(): boolean {
  try {
    return window.localStorage.getItem(LEGAL_CONSENT_STORAGE_KEY) === LEGAL_CONSENT_VERSION;
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
