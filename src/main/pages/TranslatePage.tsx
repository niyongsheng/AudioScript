import { useEffect, useState } from 'react'
import { useStore } from '@/main/store'
import { useI18n } from '@/shared/i18n'
import { type ConfigDefaults, configDefaults, TranslateEngine } from '@/shared/ipc'
import {
  isSupportedPair,
  LANGUAGES,
  type LanguageChoice,
  languageById,
  languageIdFor,
  preferredSource,
  preferredTarget,
  scriptForPair,
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
    translateEngine,
    setTranslateEngine,
    sourceLang,
    targetLang,
    zhScript,
    setDirection,
    endpoint,
    setEndpoint,
    status,
  } = useStore()
  const { t } = useI18n()
  const capturing = status === TaskStatus.Capturing

  const engineOptions = [
    { value: TranslateEngine.LocalNmt, label: t('translate.engineLocal') },
    { value: TranslateEngine.Off, label: t('translate.engineOff') },
    {
      value: TranslateEngine.RemoteApi,
      label: t('translate.engineRemote'),
      unavailableReason: t('translate.engineRemoteUnavailable'),
    },
  ]

  const mockOption = { value: TranslateEngine.Mock, label: t('translate.engineMock') }

  const sourceChoice = languageById(languageIdFor(sourceLang, zhScript)) ?? LANGUAGES[0]
  const targetChoice = languageById(languageIdFor(targetLang, zhScript)) ?? LANGUAGES[0]

  const sourceOptions = LANGUAGES.map((language) => ({
    value: language.id,
    label: t(language.labelKey),
  }))

  const targetOptions = LANGUAGES.map((language) => ({
    value: language.id,
    label: t(language.labelKey),
    disabled: !isSupportedPair(sourceChoice, language),
  }))

  const directionDisabled = capturing || translateEngine === TranslateEngine.Off

  function applyPair(source: LanguageChoice, target: LanguageChoice) {
    setDirection(source.backend, target.backend, scriptForPair(source, target, zhScript))
  }

  function changeSource(id: string) {
    const next = languageById(id)
    if (!next) return
    const target = isSupportedPair(next, targetChoice) ? targetChoice : preferredTarget(next, zhScript)
    if (target) applyPair(next, target)
  }

  function changeTarget(id: string) {
    const next = languageById(id)
    if (!next) return
    const source = isSupportedPair(sourceChoice, next) ? sourceChoice : preferredSource(next, zhScript)
    if (source) applyPair(source, next)
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
              label={t('translate.engine')}
              value={translateEngine}
              options={translateEngine === TranslateEngine.Mock ? [...engineOptions, mockOption] : engineOptions}
              onChange={setTranslateEngine}
              disabled={capturing}
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
                label={t('translate.sourceLanguage')}
                value={sourceChoice.id}
                options={sourceOptions}
                onChange={changeSource}
                disabled={directionDisabled}
              />
              <Icon name="arrowRight" size={14} className="shrink-0 text-[var(--color-ink-2)]" />
              <Select
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
