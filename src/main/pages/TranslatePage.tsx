import { useEffect, useMemo, useRef, useState } from 'react'
import { useStore } from '@/main/store'
import { useModels } from '@/main/useModels'
import { type MessageKey, useI18n } from '@/shared/i18n'
import {
  type ConfigDefaults,
  configDefaults,
  EngineType,
  formatCmdError,
  macTranslationLanguages,
  modelSelect,
  TranslateEngine,
} from '@/shared/ipc'
import {
  APPLE_TRANSLATION_FALLBACKS,
  appleTargetChoices,
  displayNameFormatter,
  isSupportedPair,
  type LanguageChoice,
  languageById,
  languageIdFor,
  languageLabel,
  languageSubtitle,
  multilingualChoices,
  preferredSource,
  preferredTarget,
  scriptForPair,
  sourceLanguagesFor,
  targetLanguagesFor,
} from '@/shared/languages'
import { formatBytes, modelLanguageLabel, modelStatusBadge, modelTitle } from '@/shared/models'
import {
  MAX_MAX_UTTERANCE_MS,
  MAX_TRAILING_SILENCE_MS,
  MIN_MAX_UTTERANCE_MS,
  MIN_TRAILING_SILENCE_MS,
  ModelKind,
  type ModelStatus,
  TaskStatus,
} from '@/shared/types'
import { Alert } from '@/ui/Alert'
import { Icon } from '@/ui/Icon'
import { Page } from '@/ui/Page'
import { Select } from '@/ui/Select'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'
import { Slider } from '@/ui/Slider'

type TranslateFn = (key: MessageKey, vars?: Record<string, string | number>) => string

function statusPart(model: ModelStatus, t: TranslateFn): string {
  if (model.complete) return t('models.installedSize', { size: formatBytes(model.installedBytes) })
  return t(modelStatusBadge(model).key)
}

const ASR_DESCRIPTION_KEYS: Record<string, MessageKey> = {
  'asr-builtin': 'translate.asrEngineBuiltinDescription',
  'asr-high-accuracy': 'translate.asrEngineHighDescription',
  'asr-qwen3': 'translate.asrEngineQwen3Description',
}

function modelSubtitle(model: ModelStatus, t: TranslateFn): string {
  const parts: string[] = []
  if (model.kind === ModelKind.Asr) {
    parts.push(t(model.streaming ? 'models.streaming' : 'models.nonStreaming'))
  }
  const language = modelLanguageLabel(model, t)
  if (language) parts.push(language)
  parts.push(statusPart(model, t))
  return parts.join(' · ')
}

export function TranslatePage() {
  const {
    info,
    translateEngine,
    setTranslateEngine,
    asrEngine,
    setAsrEngine,
    setEngineAndDirection,
    sourceLang,
    targetLang,
    zhScript,
    setDirection,
    endpoint,
    setEndpoint,
    status,
    setError,
  } = useStore()
  const { locale, t } = useI18n()
  const { list: models, setList: setModels } = useModels()
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

  const displayName = useMemo(() => displayNameFormatter(locale), [locale])
  const appleChoices = useMemo(() => appleTargetChoices(appleLanguages, displayName), [appleLanguages, displayName])
  const selectedAsrModel = models?.asr.find((model) => model.selected) ?? null
  const selectedMtModel = models?.mt.find((model) => model.selected) ?? models?.mt[0] ?? null
  const multilingualCatalog = useMemo(
    () => multilingualChoices(selectedMtModel?.languages ?? [], displayName),
    [selectedMtModel, displayName],
  )
  const multilingualAsr = selectedAsrModel?.multilingual === true
  const multilingualMt = selectedMtModel?.multilingual === true
  const targetCatalog = targetLanguagesFor(translateEngine, appleChoices, multilingualMt ? multilingualCatalog : [])
  const sourceCatalog = sourceLanguagesFor(
    translateEngine,
    multilingualAsr || asrEngine === EngineType.MacNative,
    appleChoices,
    multilingualMt ? multilingualCatalog : [],
  )

  const localModelOptions = (list: ModelStatus[], subtitle: (model: ModelStatus) => string) =>
    list.map((model) => ({
      value: `local:${model.id}`,
      label: modelTitle(model, t),
      subtitle: subtitle(model),
      disabled: !model.complete,
    }))

  const macAsrOption = info?.macSpeechSupported
    ? [
        {
          value: EngineType.MacNative,
          label: t('translate.asrEngineMac'),
          subtitle: t('translate.asrEngineMacSubtitle'),
        },
      ]
    : []

  const asrOptions = models
    ? [
        ...localModelOptions(models.asr, (model) => `${t('translate.asrEngineLocal')} · ${modelSubtitle(model, t)}`),
        ...macAsrOption,
      ]
    : [
        {
          value: EngineType.LocalStreaming,
          label: t('translate.asrEngineLocal'),
          subtitle: t('common.loading'),
        },
        ...macAsrOption,
      ]

  const localTranslationOptions = models?.mt?.length
    ? localModelOptions(models.mt, (model) => modelSubtitle(model, t))
    : [
        {
          value: TranslateEngine.LocalNmt,
          label: t('translate.engineLocal'),
          subtitle: `${t('models.languageZh')} · ${t('models.languageEn')} · ${t('models.bundled')}`,
        },
      ]

  const translationOptions = [
    ...localTranslationOptions,
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
    ...(translateEngine === TranslateEngine.Mock
      ? [
          {
            value: TranslateEngine.Mock,
            label: t('translate.engineMock'),
            subtitle: t('translate.engineMockSubtitle'),
          },
        ]
      : []),
  ]

  const translationValue =
    translateEngine === TranslateEngine.LocalNmt && selectedMtModel ? `local:${selectedMtModel.id}` : translateEngine

  const sourceChoice =
    languageById(languageIdFor(sourceLang, zhScript, sourceCatalog), sourceCatalog) ?? sourceCatalog[0]
  const targetChoice =
    languageById(languageIdFor(targetLang, zhScript, targetCatalog), targetCatalog) ?? targetCatalog[0]

  const sourceOptions = sourceCatalog.map((language) => ({
    value: language.id,
    label: languageLabel(language, t),
    subtitle: languageSubtitle(language),
  }))

  const targetOptions = targetCatalog.map((language) => ({
    value: language.id,
    label: languageLabel(language, t),
    subtitle: languageSubtitle(language),
    disabled: !isSupportedPair(sourceChoice, language, translateEngine, multilingualMt),
  }))

  const directionDisabled = capturing || translateEngine === TranslateEngine.Off
  const directionSubtitle = (() => {
    switch (translateEngine) {
      case TranslateEngine.Off:
        return t('translate.directionSubtitleOff')
      case TranslateEngine.MacTranslation:
        return t('translate.directionSubtitleMac')
      case TranslateEngine.Mock:
        return t('translate.engineMockSubtitle')
      case TranslateEngine.LocalNmt:
        return multilingualMt ? t('translate.directionSubtitleMultilingual') : t('translate.directionSubtitleLocal')
      default:
        return t('translate.directionSubtitleLocal')
    }
  })()

  useEffect(() => {
    if (!models) return
    if (languageById(languageIdFor(sourceLang, zhScript, sourceCatalog), sourceCatalog)) return
    const fallback =
      preferredSource(targetChoice, zhScript, translateEngine, sourceCatalog, multilingualMt) ?? sourceCatalog[0]
    setDirection(fallback.backend, targetChoice.backend, scriptForPair(fallback, targetChoice, zhScript))
  }, [models, sourceLang, zhScript, sourceCatalog, targetChoice, translateEngine, multilingualMt, setDirection])

  const selectedAsr = selectedAsrModel?.id ?? models?.asr[0]?.id ?? ''
  const macNative = asrEngine === EngineType.MacNative
  const asrValue = macNative ? EngineType.MacNative : selectedAsr ? `local:${selectedAsr}` : EngineType.LocalStreaming
  const segmentationDisabled = capturing || macNative || selectedAsrModel?.streaming === false
  const asrDescription = macNative
    ? t('translate.asrEngineMacDescription')
    : selectedAsrModel
      ? t(ASR_DESCRIPTION_KEYS[selectedAsrModel.id] ?? 'translate.asrEngineDescription')
      : t('translate.asrEngineDescription')

  async function changeModel(id: string): Promise<boolean> {
    try {
      setModels(await modelSelect(id))
      return true
    } catch (e) {
      setError(formatCmdError(e))
      return false
    }
  }

  async function changeAsr(value: string) {
    if (value === EngineType.MacNative) {
      setAsrEngine(EngineType.MacNative)
      return
    }
    const id = value.startsWith('local:') ? value.slice('local:'.length) : ''
    if (!id) return
    if (macNative) setAsrEngine(EngineType.LocalStreaming)
    await changeModel(id)
  }

  function applyPair(source: LanguageChoice, target: LanguageChoice) {
    setDirection(source.backend, target.backend, scriptForPair(source, target, zhScript))
  }

  function changeSource(id: string) {
    const next = languageById(id, sourceCatalog)
    if (!next) return
    const target = isSupportedPair(next, targetChoice, translateEngine, multilingualMt)
      ? targetChoice
      : preferredTarget(next, zhScript, translateEngine, targetCatalog, multilingualMt)
    if (target) applyPair(next, target)
  }

  function changeTarget(id: string) {
    const next = languageById(id, targetCatalog)
    if (!next) return
    const source = isSupportedPair(sourceChoice, next, translateEngine, multilingualMt)
      ? sourceChoice
      : preferredSource(next, zhScript, translateEngine, sourceCatalog, multilingualMt)
    if (source) applyPair(source, next)
  }

  async function changeTranslation(value: string) {
    const local = value.startsWith('local:')
    const engine = local ? TranslateEngine.LocalNmt : (value as TranslateEngine)
    const nextModel = local
      ? (models?.mt.find((model) => model.id === value.slice('local:'.length)) ?? null)
      : selectedMtModel

    if (local && nextModel && nextModel.id !== selectedMtModel?.id && !(await changeModel(nextModel.id))) {
      return
    }

    const pairs = engine === TranslateEngine.LocalNmt && nextModel?.multilingual === true
    const multilingual = pairs && nextModel ? multilingualChoices(nextModel.languages, displayName) : []
    const nextTargets = targetLanguagesFor(engine, appleChoices, multilingual)
    const nextSources = sourceLanguagesFor(
      engine,
      multilingualAsr || asrEngine === EngineType.MacNative,
      appleChoices,
      multilingual,
    )
    const source =
      languageById(languageIdFor(sourceLang, zhScript, nextSources), nextSources) ??
      preferredSource(targetChoice, zhScript, engine, nextSources, pairs) ??
      nextSources[0]
    const target = isSupportedPair(source, targetChoice, engine, pairs)
      ? targetChoice
      : preferredTarget(source, zhScript, engine, nextTargets, pairs)
    if (!target) {
      setTranslateEngine(engine)
      return
    }
    setEngineAndDirection(engine, source.backend, target.backend, scriptForPair(source, target, zhScript))
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
              value={translationValue}
              options={translationOptions}
              onChange={(value) => void changeTranslation(value)}
              disabled={capturing}
              className="w-[16rem]"
            />
          }
        />
        <SettingsRow
          icon="translate"
          title={t('translate.direction')}
          description={directionSubtitle}
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

      <SettingsGroup title={t('models.asrGroup')} footer={t('translate.footer')}>
        <SettingsRow
          icon="waveform"
          title={t('translate.asrEngineTitle')}
          description={asrDescription}
          control={
            <Select
              variant="rich"
              label={t('translate.asrEngineTitle')}
              value={asrValue}
              options={asrOptions}
              onChange={(value) => void changeAsr(value)}
              disabled={capturing}
              className="w-[16rem]"
            />
          }
        />
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
                disabled={segmentationDisabled}
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
                disabled={segmentationDisabled}
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
                disabled={segmentationDisabled || !defaults}
                onClick={() => defaults && void setEndpoint(defaults.endpointProfiles.interpretation)}
                className="rounded-[var(--radius-ctl)] border border-[var(--color-border-subtle)] px-2 py-1 text-[var(--color-ink-1)] text-xs transition hover:bg-[var(--color-surface-2)] disabled:opacity-40"
              >
                {t('translate.presetInterpretation')}
              </button>
              <button
                type="button"
                disabled={segmentationDisabled || !defaults}
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
