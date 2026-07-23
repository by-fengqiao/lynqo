import { readonly, shallowRef } from "vue";
import { messages, type Locale } from "./messages";

const storageKey = "lynqo.locale";
const supportedLocales: Locale[] = ["zh-CN", "en-US"];

function readStoredLocale(): Locale {
  if (typeof window === "undefined") return "zh-CN";
  const stored = window.localStorage.getItem(storageKey);
  return supportedLocales.includes(stored as Locale) ? (stored as Locale) : "zh-CN";
}

const activeLocale = shallowRef<Locale>(readStoredLocale());

function applyDocumentLocale(locale: Locale) {
  if (typeof document === "undefined") return;
  document.documentElement.lang = locale;
}

function interpolate(message: string, params: Record<string, string | number>) {
  return message.replace(/\{(\w+)\}/g, (match, key: string) =>
    params[key] === undefined ? match : String(params[key])
  );
}

export function translate(key: string, params: Record<string, string | number> = {}) {
  const message = messages[activeLocale.value][key] ?? messages["zh-CN"][key] ?? key;
  return interpolate(message, params);
}

export function setLocale(locale: Locale) {
  activeLocale.value = locale;
  if (typeof window !== "undefined") {
    window.localStorage.setItem(storageKey, locale);
  }
  applyDocumentLocale(locale);
}

export function initializeLocale() {
  applyDocumentLocale(activeLocale.value);
}

export function getCurrentLocale(): Locale {
  return activeLocale.value;
}

export function useLocale() {
  return {
    locale: readonly(activeLocale),
    setLocale,
    t: translate,
  };
}

export type { Locale } from "./messages";
