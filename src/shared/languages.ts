import type { MessageKey } from '@/shared/i18n'
import { ZhScript } from '@/shared/types'

export interface LanguageChoice {
  id: string
  labelKey: MessageKey
  backend: string
  script?: ZhScript
}

export const LANGUAGES: readonly LanguageChoice[] = [
  { id: 'en', labelKey: 'translate.langEnglish', backend: 'en' },
  { id: 'zh-CN', labelKey: 'translate.langZhCn', backend: 'zh', script: ZhScript.Simplified },
  { id: 'zh-TW', labelKey: 'translate.langZhTw', backend: 'zh', script: ZhScript.Traditional },
]

export const SUPPORTED_PAIRS: ReadonlyArray<readonly [string, string]> = [
  ['en', 'zh'],
  ['zh', 'en'],
]

export function languageById(id: string): LanguageChoice | undefined {
  return LANGUAGES.find((language) => language.id === id)
}

export function languageIdFor(backend: string, zhScript: ZhScript): string {
  if (backend === 'zh') return zhScript === ZhScript.Traditional ? 'zh-TW' : 'zh-CN'
  return LANGUAGES.find((language) => language.backend === backend)?.id ?? backend
}

export function isSupportedPair(source: LanguageChoice, target: LanguageChoice): boolean {
  return SUPPORTED_PAIRS.some(([from, to]) => from === source.backend && to === target.backend)
}

function preferredBy(matches: (language: LanguageChoice) => boolean, zhScript: ZhScript): LanguageChoice | undefined {
  const candidates = LANGUAGES.filter(matches)
  return candidates.find((language) => language.script === zhScript) ?? candidates[0]
}

export function preferredTarget(source: LanguageChoice, zhScript: ZhScript): LanguageChoice | undefined {
  return preferredBy((language) => isSupportedPair(source, language), zhScript)
}

export function preferredSource(target: LanguageChoice, zhScript: ZhScript): LanguageChoice | undefined {
  return preferredBy((language) => isSupportedPair(language, target), zhScript)
}

export function scriptForPair(source: LanguageChoice, target: LanguageChoice, fallback: ZhScript): ZhScript {
  return source.script ?? target.script ?? fallback
}
