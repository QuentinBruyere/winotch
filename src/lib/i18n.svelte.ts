// Translations (ADR-0012): the same flat JSON files as the Rust side. The
// language is decided by the backend and arrives with the status / settings.
import de from '../../locales/de.json'
import en from '../../locales/en.json'
import es from '../../locales/es.json'
import fr from '../../locales/fr.json'
import ja from '../../locales/ja.json'

export type Key = keyof typeof en

const FALLBACK = 'en'
// Mirrors `i18n::BUILT_IN` in src-tauri/src/i18n.rs
const catalogs: Record<string, Record<string, string>> = {
  de: { ...de },
  en: { ...en },
  es: { ...es },
  fr: { ...fr },
  ja: { ...ja },
}

let language = $state(FALLBACK)

export function setLanguage(code: string) {
  language = code
  document.documentElement.lang = code
}

// For a language's `Intl` formatting (numbers, lists…).
export function currentLanguage(): string {
  return language
}

// A module's own translations (its settings component), keys prefixed with
// the module id.
export function registerLocales(code: string, entries: Record<string, string>) {
  catalogs[code] = { ...catalogs[code], ...entries }
}

type Params = Record<string, string | number>

// The text of `key` in the current language, `{name}` placeholders filled.
// Reactive: a component showing it updates when the language changes.
export function t(key: Key, params: Params = {}): string {
  return translate(key, params)
}

// Like `t`, for any key, such as a module's own (`registerLocales`). A key
// missing from the language falls back on English, then on the key itself.
export function translate(key: string, params: Params = {}): string {
  const text = catalogs[language]?.[key] ?? catalogs[FALLBACK][key] ?? key
  return text.replace(/\{(\w+)\}/g, (all, name: string) =>
    name in params ? String(params[name]) : all,
  )
}
