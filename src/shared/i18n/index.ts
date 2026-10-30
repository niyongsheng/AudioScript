import { useSyncExternalStore } from 'react'
import { ar } from './ar'
import { de } from './de'
import { en } from './en'
import { es } from './es'
import { fr } from './fr'
import { id as idLocale } from './id'
import { it } from './it'
import { ja } from './ja'
import { ko } from './ko'
import { ptBR } from './pt-BR'
import { ru } from './ru'
import { type Messages, zhCN } from './zh-CN'
import { zhTW } from './zh-TW'

const CATALOGS = {
  'zh-CN': zhCN,
  'zh-TW': zhTW,
  en,
  ja,
  ko,
  fr,
  de,
  es,
  'pt-BR': ptBR,
  ru,
  it,
  id: idLocale,
  ar,
}

export type Locale = keyof typeof CATALOGS

export type MessageKey = keyof Messages

type Vars = Record<string, string | number>

export const LOCALES = Object.keys(CATALOGS) as readonly Locale[]

export function isLocale(value: string): value is Locale {
  return value in CATALOGS
}

let current: Locale = 'en'

const listeners = new Set<() => void>()

function applyDocumentLang() {
  if (typeof document === 'undefined') return
  document.documentElement.lang = current
  document.documentElement.dir = current === 'ar' ? 'rtl' : 'ltr'
  document.title = t('app.windowTitle')
}

applyDocumentLang()

export function getLocale(): Locale {
  return current
}

export function setLocale(next: Locale) {
  if (!isLocale(next) || next === current) return
  current = next
  applyDocumentLang()
  for (const listener of listeners) listener()
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export function t(key: MessageKey, vars?: Vars): string {
  const template = CATALOGS[current][key] ?? en[key] ?? zhCN[key] ?? key
  if (!vars) return template
  return template.replace(/\{(\w+)\}/g, (match, name: string) => (name in vars ? String(vars[name]) : match))
}

export function useI18n(): { locale: Locale; t: typeof t } {
  const locale = useSyncExternalStore(subscribe, getLocale, getLocale)
  return { locale, t }
}
