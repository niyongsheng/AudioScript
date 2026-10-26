import { useSyncExternalStore } from 'react'
import { en } from './en'
import { ja } from './ja'
import { type Messages, zhCN } from './zh-CN'
import { zhTW } from './zh-TW'

export type Locale = 'zh-CN' | 'zh-TW' | 'en' | 'ja'

export type MessageKey = keyof Messages

type Vars = Record<string, string | number>

const CATALOGS: Record<Locale, Messages> = {
  'zh-CN': zhCN,
  'zh-TW': zhTW,
  en,
  ja,
}

export const LOCALES: readonly Locale[] = ['zh-CN', 'zh-TW', 'en', 'ja']

export function isLocale(value: string): value is Locale {
  return (LOCALES as readonly string[]).includes(value)
}

let current: Locale = 'en'

const listeners = new Set<() => void>()

function applyDocumentLang() {
  if (typeof document === 'undefined') return
  document.documentElement.lang = current
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
  const template = CATALOGS[current][key] ?? zhCN[key] ?? key
  if (!vars) return template
  return template.replace(/\{(\w+)\}/g, (match, name: string) => (name in vars ? String(vars[name]) : match))
}

export function useI18n(): { locale: Locale; t: typeof t } {
  const locale = useSyncExternalStore(subscribe, getLocale, getLocale)
  return { locale, t }
}
