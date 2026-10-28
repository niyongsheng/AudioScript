import type { MessageKey } from '@/shared/i18n'
import { TranslateEngine, ZhScript } from '@/shared/types'

export interface LanguageChoice {
  id: string
  labelKey?: MessageKey
  label?: string
  subtitle?: string
  backend: string
  script?: ZhScript
}

export const LANGUAGES: readonly LanguageChoice[] = [
  { id: 'en', labelKey: 'translate.langEnglish', backend: 'en', subtitle: 'en' },
  {
    id: 'zh-CN',
    labelKey: 'translate.langZhCn',
    backend: 'zh',
    subtitle: 'zh-Hans',
    script: ZhScript.Simplified,
  },
  {
    id: 'zh-TW',
    labelKey: 'translate.langZhTw',
    backend: 'zh',
    subtitle: 'zh-Hant',
    script: ZhScript.Traditional,
  },
  { id: 'ja', labelKey: 'translate.langJapanese', backend: 'ja', subtitle: 'ja' },
  { id: 'ko', labelKey: 'translate.langKorean', backend: 'ko', subtitle: 'ko' },
  { id: 'fr', labelKey: 'translate.langFrench', backend: 'fr', subtitle: 'fr' },
]

export const SOURCE_LANGUAGES: readonly LanguageChoice[] = LANGUAGES.filter(
  (language) => language.backend === 'en' || language.backend === 'zh',
)

export const SUPPORTED_PAIRS: ReadonlyArray<readonly [string, string]> = [
  ['en', 'zh'],
  ['zh', 'en'],
]

export const APPLE_TRANSLATION_FALLBACKS: readonly string[] = [
  'ar-Arab-AE',
  'de-Latn-DE',
  'en-Latn-GB',
  'en-Latn-US',
  'es-Latn-ES',
  'fr-Latn-FR',
  'hi-Deva-IN',
  'id-Latn-ID',
  'it-Latn-IT',
  'ja-Jpan-JP',
  'ko-Kore-KR',
  'nl-Latn-NL',
  'pl-Latn-PL',
  'pt-Latn-BR',
  'ru-Cyrl-RU',
  'th-Thai-TH',
  'tr-Latn-TR',
  'uk-Cyrl-UA',
  'vi-Latn-VN',
  'zh-Hans-CN',
  'zh-Hant-TW',
]

export function baseLanguage(tag: string): string {
  return tag.split('-')[0] ?? tag
}

export function languageLabel(language: LanguageChoice, t: (key: MessageKey) => string): string {
  return language.labelKey ? t(language.labelKey) : (language.label ?? language.id)
}

export function languageSubtitle(language: LanguageChoice): string {
  return language.subtitle ?? language.backend
}

export function targetLanguagesFor(
  engine: TranslateEngine,
  appleChoices: readonly LanguageChoice[] = [],
): readonly LanguageChoice[] {
  if (engine !== TranslateEngine.MacTranslation) return SOURCE_LANGUAGES
  return appleChoices.length > 0 ? appleChoices : LANGUAGES
}

export function appleTargetChoices(
  identifiers: readonly string[],
  displayName: (code: string) => string,
): LanguageChoice[] {
  const byId = new Map<string, LanguageChoice>()

  for (const identifier of identifiers) {
    const base = baseLanguage(identifier)
    const choice = staticChoiceFor(base, identifier, displayName)
    if (!choice || byId.has(choice.id)) continue
    byId.set(choice.id, { ...choice, subtitle: identifier })
  }

  return [...byId.values()]
}

function staticChoiceFor(
  base: string,
  identifier: string,
  displayName: (code: string) => string,
): LanguageChoice | undefined {
  if (base === 'zh') {
    const traditional = identifier.includes('Hant') || identifier.includes('TW') || identifier.includes('HK')
    return LANGUAGES.find((language) => language.id === (traditional ? 'zh-TW' : 'zh-CN'))
  }
  const known = LANGUAGES.find((language) => language.backend === base)
  if (known) return known
  return { id: base, label: displayName(base), backend: base }
}

export function languageById(id: string, catalog: readonly LanguageChoice[] = LANGUAGES): LanguageChoice | undefined {
  return catalog.find((language) => language.id === id)
}

export function languageIdFor(
  backend: string,
  zhScript: ZhScript,
  catalog: readonly LanguageChoice[] = LANGUAGES,
): string {
  if (backend === 'zh') return zhScript === ZhScript.Traditional ? 'zh-TW' : 'zh-CN'
  return (
    catalog.find((language) => language.id === backend)?.id ??
    catalog.find((language) => language.backend === backend)?.id ??
    backend
  )
}

export function isSupportedPair(source: LanguageChoice, target: LanguageChoice, engine: TranslateEngine): boolean {
  const sourceBase = baseLanguage(source.backend)
  const targetBase = baseLanguage(target.backend)
  if (sourceBase === targetBase) return false

  if (engine === TranslateEngine.MacTranslation) {
    return sourceBase === 'en' || sourceBase === 'zh'
  }

  return SUPPORTED_PAIRS.some(([from, to]) => from === sourceBase && to === targetBase)
}

function preferredBy(
  matches: (language: LanguageChoice) => boolean,
  zhScript: ZhScript,
  catalog: readonly LanguageChoice[],
): LanguageChoice | undefined {
  const candidates = catalog.filter(matches)
  return candidates.find((language) => language.script === zhScript) ?? candidates[0]
}

export function preferredTarget(
  source: LanguageChoice,
  zhScript: ZhScript,
  engine: TranslateEngine,
  catalog: readonly LanguageChoice[] = LANGUAGES,
): LanguageChoice | undefined {
  return preferredBy((language) => isSupportedPair(source, language, engine), zhScript, catalog)
}

export function preferredSource(
  target: LanguageChoice,
  zhScript: ZhScript,
  engine: TranslateEngine,
  catalog: readonly LanguageChoice[] = SOURCE_LANGUAGES,
): LanguageChoice | undefined {
  return preferredBy((language) => isSupportedPair(language, target, engine), zhScript, catalog)
}

export function scriptForPair(source: LanguageChoice, target: LanguageChoice, fallback: ZhScript): ZhScript {
  return source.script ?? target.script ?? fallback
}
