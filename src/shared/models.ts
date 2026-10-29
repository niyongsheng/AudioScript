import type { MessageKey } from './i18n'
import type { ModelStatus } from './types'

const TITLE_KEYS: Record<string, MessageKey> = {
  'asr-builtin': 'models.modelAsrBuiltinTitle',
  'asr-high-accuracy': 'models.modelAsrHighTitle',
  'asr-qwen3': 'models.modelAsrQwen3Title',
  'mt-base': 'models.modelMtBaseTitle',
  'mt-multilingual': 'models.modelMtMultilingualTitle',
}

const LANGUAGE_KEYS: Record<string, MessageKey> = {
  zh: 'models.languageZh',
  en: 'models.languageEn',
}

const LANGUAGE_PAIRS: Record<string, MessageKey> = {
  'en,zh': 'models.languagesZhEn',
}

export function modelTitle(model: { id: string; title: string }, t: (key: MessageKey) => string): string {
  const key = TITLE_KEYS[model.id]
  return key ? t(key) : model.title
}

function languageLabel(codes: readonly string[], t: (key: MessageKey) => string): string {
  const pair = LANGUAGE_PAIRS[[...codes].sort().join(',')]
  if (pair) return t(pair)
  return codes
    .map((code) => {
      const key = LANGUAGE_KEYS[code]
      return key ? t(key) : code.toUpperCase()
    })
    .join(' · ')
}

export type ModelStatusTone = 'ok' | 'warn' | 'danger' | 'muted'

export function modelStatusBadge(model: ModelStatus): { tone: ModelStatusTone; key: MessageKey } {
  if (model.complete) {
    return model.bundled ? { tone: 'muted', key: 'models.bundled' } : { tone: 'ok', key: 'models.statusDownloaded' }
  }
  if (model.bundled) return { tone: 'danger', key: 'models.statusBundledMissing' }
  if (model.installed) return { tone: 'danger', key: 'models.statusIncomplete' }
  if (model.downloadable) return { tone: 'warn', key: 'models.statusNotInstalled' }
  return { tone: 'muted', key: 'models.statusNotPublished' }
}

export function modelLanguageLabel(model: ModelStatus, t: (key: MessageKey) => string): string | null {
  if (model.multilingual) return t('models.multilingual')
  if (model.languages.length > 0) return languageLabel(model.languages, t)
  return null
}

export function formatBytes(bytes: number): string {
  if (!bytes) return '—'
  const mb = bytes / (1024 * 1024)
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`
  if (mb >= 10) return `${Math.round(mb)} MB`
  return `${mb.toFixed(1)} MB`
}
