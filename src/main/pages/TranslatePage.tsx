import { useEffect, useMemo, useRef, useState } from 'react'
import { useStore } from '@/main/store'
import { useI18n } from '@/shared/i18n'
import { type ConfigDefaults, configDefaults, macTranslationLanguages, TranslateEngine } from '@/shared/ipc'
import {
  APPLE_TRANSLATION_FALLBACKS,
  appleTargetChoices,
  isSupportedPair,
  type LanguageChoice,
  languageById,
  languageIdFor,
  languageLabel,
  languageSubtitle,
  preferredSource,
  preferredTarget,
  SOURCE_LANGUAGES,
  scriptForPair,
  targetLanguagesFor,
} from '@/shared/languages'
import {
  MAX_MAX_UTTERANCE_MS,
  MAX_TRAILING_SILENCE_MS,
  MIN_MAX_UTTERANCE_MS,
  MIN_TRAILING_SILENCE_MS,
  TaskStatus,
} from '@/shared/types'
import { Alert } from '@/ui/Alert'
import { Icon } from '@/ui/Icon'
import { Page } from '@/ui/Page'
import { Select } from '@/ui/Select'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'
import { Slider } from '@/ui/Slider'

export function TranslatePage() {
  const {
    info,
    translateEngine,
    setTranslateEngine,
    setEngineAndDirection,
    sourceLang,
    targetLang,
    zhScript,
    setDirection,
    endpoint,
    setEndpoint,
    status,
  } = useStore()
  const { locale, t } = useI18n()
  const capturing = status === TaskStatus.Capturing
  const macMajor = Number.parseInt(info?.osVersion?.split('.')[0] ?? '', 10)
  const macEngineAvailable = info?.platform === 'macos' && macMajor >= 26
  const [appleLanguages, setAppleLanguages] = useState<string[]>(() => [...APPLE_TRANSLATION_FALLBACKS])
  const appleLanguagesLoaded = useRef(false)

  useEffect(() => {
    if (!macEngineAvailable || translateEngine !== TranslateEngine.MacTranslation || appleLanguagesLoaded.current) {
      return
    }
    let disposed = false
    macTranslationLanguages()
      .then((languages) => {
        if (disposed || languages.length === 0) return
        appleLanguagesLoaded.current = true
        setAppleLanguages(languages)
      })
      .catch(() => {})
    return () => {
      disposed = true
    }
  }, [macEngineAvailable, translateEngine])

  const appleChoices = useMemo(() => {
    let names: Intl.DisplayNames | null = null
    try {
      names = new Intl.DisplayNames([locale], { type: 'language' })
    } catch {
      names = null
    }
    return appleTargetChoices(appleLanguages, (code) => names?.of(code) ?? code)
  }, [appleLanguages, locale])
  const targetCatalog = targetLanguagesFor(translateEngine, appleChoices)

  const engineOptions = [
    {
      value: TranslateEngine.LocalNmt,
      label: t('translate.engineLocal'),
      subtitle: t('translate.engineLocalSubtitle'),
    },
    ...(macEngineAvailable
      ? [
          {
            value: TranslateEngine.MacTranslation,
            label: t('translate.engineMac'),
            subtitle: t('translate.engineMacSubtitle'),
          },
        ]
      : []),
    {
      value: TranslateEngine.Off,
      label: t('translate.engineOff'),
      subtitle: t('translate.engineOffSubtitle'),
    },
    {
      value: TranslateEngine.RemoteApi,
      label: t('translate.engineRemote'),
      subtitle: t('translate.engineRemoteUnavailable'),
      disabled: true,
    },
  ]

  const mockOption = {
    value: TranslateEngine.Mock,
    label: t('translate.engineMock'),
    subtitle: t('translate.engineMockSubtitle'),
  }

  const sourceChoice =
    languageById(languageIdFor(sourceLang, zhScript, SOURCE_LANGUAGES), SOURCE_LANGUAGES) ?? SOURCE_LANGUAGES[0]
  const targetChoice =
    languageById(languageIdFor(targetLang, zhScript, targetCatalog), targetCatalog) ?? targetCatalog[0]

  const sourceOptions = SOURCE_LANGUAGES.map((language) => ({
    value: language.id,
    label: languageLabel(language, t),
    subtitle: languageSubtitle(language),
  }))

  const targetOptions = targetCatalog.map((language) => ({
    value: language.id,
    label: languageLabel(language, t),
    subtitle: languageSubtitle(language),
    disabled: !isSupportedPair(sourceChoice, language, translateEngine),
  }))

  const directionDisabled = capturing || translateEngine === TranslateEngine.Off

  function applyPair(source: LanguageChoice, target: LanguageChoice) {
    setDirection(source.backend, target.backend, scriptForPair(source, target, zhScript))
  }

  function changeSource(id: string) {
    const next = languageById(id, SOURCE_LANGUAGES)
    if (!next) return
    const target = isSupportedPair(next, targetChoice, translateEngine)
      ? targetChoice
      : preferredTarget(next, zhScript, translateEngine, targetCatalog)
    if (target) applyPair(next, target)
  }

  function changeTarget(id: string) {
    const next = languageById(id, targetCatalog)
    if (!next) return
    const source = isSupportedPair(sourceChoice, next, translateEngine)
      ? sourceChoice
      : preferredSource(next, zhScript, translateEngine, SOURCE_LANGUAGES)
    if (source) applyPair(source, next)
  }

  function changeEngine(engine: TranslateEngine) {
    const nextTargets = targetLanguagesFor(engine, appleChoices)
    const target = isSupportedPair(sourceChoice, targetChoice, engine)
      ? targetChoice
      : preferredTarget(sourceChoice, zhScript, engine, nextTargets)
    if (!target) {
      setTranslateEngine(engine)
      return
    }
    setEngineAndDirection(engine, sourceChoice.backend, target.backend, scriptForPair(sourceChoice, target, zhScript))
  }

  const [defaults, setDefaults] = useState<ConfigDefaults | null>(null)
  useEffect(() => {
    configDefaults()
      .then(setDefaults)
      .catch(() => {})
  }, [])

  const [draft, setDraft] = useState(endpoint)
  useEffect(() => setDraft(endpoint), [endpoint])

  return (
    <Page title={t('translate.title')}>
      {capturing && <Alert tone="info">{t('translate.capturingHint')}</Alert>}

      <SettingsGroup title={t('translate.engineGroup')}>
        <SettingsRow
          icon="translate"
          title={t('translate.engine')}
          description={t('translate.engineDescription')}
          control={
            <Select
              variant="rich"
              label={t('translate.engine')}
              value={translateEngine}
              options={translateEngine === TranslateEngine.Mock ? [...engineOptions, mockOption] : engineOptions}
              onChange={changeEngine}
              disabled={capturing}
              className="w-[14rem]"
            />
          }
        />
        <SettingsRow
          icon="translate"
          title={t('translate.direction')}
          description={t('translate.directionDescription')}
          control={
            <div className="flex items-center gap-1.5">
              <Select
                variant="rich"
                label={t('translate.sourceLanguage')}
                value={sourceChoice.id}
                options={sourceOptions}
                onChange={changeSource}
                disabled={directionDisabled}
              />
              <Icon name="arrowRight" size={14} className="shrink-0 text-[var(--color-ink-2)]" />
              <Select
                variant="rich"
                label={t('translate.targetLanguage')}
                value={targetChoice.id}
                options={targetOptions}
                onChange={changeTarget}
                disabled={directionDisabled}
              />
            </div>
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t('translate.segmentation')} footer={t('translate.footer')}>
        <SettingsRow
          icon="waveform"
          title={t('translate.trailingTitle')}
          description={t('translate.trailingDescription')}
          control={
            <div className="w-64">
              <Slider
                label={t('translate.trailingLabel')}
                value={draft.trailingSilenceMs}
                min={MIN_TRAILING_SILENCE_MS}
                max={MAX_TRAILING_SILENCE_MS}
                step={50}
                format={(v) => `${v} ms`}
                onChange={(v) => setDraft((d) => ({ ...d, trailingSilenceMs: v }))}
                onCommit={(v) => void setEndpoint({ ...draft, trailingSilenceMs: v })}
                defaultValue={defaults?.endpoint.trailingSilenceMs}
                disabled={capturing}
              />
            </div>
          }
        />
        <SettingsRow
          icon="alert"
          title={t('translate.maxTitle')}
          description={t('translate.maxDescription')}
          control={
            <div className="w-64">
              <Slider
                label={t('translate.maxLabel')}
                value={draft.maxUtteranceMs}
                min={MIN_MAX_UTTERANCE_MS}
                max={MAX_MAX_UTTERANCE_MS}
                step={500}
                format={(v) => `${(v / 1000).toFixed(1)} s`}
                onChange={(v) => setDraft((d) => ({ ...d, maxUtteranceMs: v }))}
                onCommit={(v) => void setEndpoint({ ...draft, maxUtteranceMs: v })}
                defaultValue={defaults?.endpoint.maxUtteranceMs}
                disabled={capturing}
              />
            </div>
          }
        />
        <SettingsRow
          icon="chip"
          title={t('translate.preset')}
          control={
            <div className="flex gap-1.5">
              <button
                type="button"
                disabled={capturing || !defaults}
                onClick={() => defaults && void setEndpoint(defaults.endpointProfiles.interpretation)}
                className="rounded-[var(--radius-ctl)] border border-[var(--color-border-subtle)] px-2 py-1 text-[var(--color-ink-1)] text-xs transition hover:bg-[var(--color-surface-2)] disabled:opacity-40"
              >
                {t('translate.presetInterpretation')}
              </button>
              <button
                type="button"
                disabled={capturing || !defaults}
                onClick={() => defaults && void setEndpoint(defaults.endpointProfiles.dictation)}
                className="rounded-[var(--radius-ctl)] border border-[var(--color-border-subtle)] px-2 py-1 text-[var(--color-ink-1)] text-xs transition hover:bg-[var(--color-surface-2)] disabled:opacity-40"
              >
                {t('translate.presetDictation')}
              </button>
            </div>
          }
        />
      </SettingsGroup>
    </Page>
  )
}
