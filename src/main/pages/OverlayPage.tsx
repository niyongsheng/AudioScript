import { useEffect, useRef, useState } from 'react'
import { useStore } from '@/main/store'
import { hotkeyLabel } from '@/shared/hotkeys'
import { useI18n } from '@/shared/i18n'
import { formatCmdError, type OverlayConfig, overlaySetMode, shortcutsSet } from '@/shared/ipc'
import { isMacOS } from '@/shared/platform'
import { MAX_FONT_SIZE, MAX_GLOW_SPEED, MAX_OVERLAY_LINES, MIN_FONT_SIZE, OverlayMode } from '@/shared/types'
import { Button } from '@/ui/Button'
import { Page } from '@/ui/Page'
import { Select } from '@/ui/Select'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'
import { ShortcutRecorder } from '@/ui/ShortcutRecorder'
import { Slider } from '@/ui/Slider'
import { Switch } from '@/ui/Switch'

export function OverlayPage() {
  const { overlayCfg, overlayVisible, toggleOverlay, updateOverlay, hotkeys, refreshHotkeys, setError } = useStore()
  const { t } = useI18n()

  const [draft, setDraft] = useState<OverlayConfig | null>(overlayCfg)
  useEffect(() => setDraft(overlayCfg), [overlayCfg])

  if (!overlayCfg || !draft) {
    return (
      <Page title={t('overlay.title')}>
        <p className="text-[var(--color-ink-2)] text-sm">{t('overlay.loading')}</p>
      </Page>
    )
  }

  async function rebind(id: string, accelerator: string) {
    try {
      await shortcutsSet(id, accelerator)
      await refreshHotkeys()
    } catch (e) {
      setError(formatCmdError(e))
    }
  }

  return (
    <Page title={t('overlay.title')}>
      <SettingsGroup title={t('overlay.displayGroup')}>
        <SettingsRow
          icon="overlay"
          title={overlayVisible ? t('overlay.hideOverlay') : t('overlay.showOverlay')}
          control={
            <Button onClick={() => void toggleOverlay()}>
              {overlayVisible ? t('overlay.hide') : t('overlay.show')}
            </Button>
          }
        />
        <SettingsRow
          icon="keyboard"
          title={t('overlay.mode')}
          description={
            overlayCfg.mode === OverlayMode.Locked
              ? t('overlay.modeLockedDescription')
              : t('overlay.modeEditDescription')
          }
          control={
            <Select
              label={t('overlay.mode')}
              value={overlayCfg.mode}
              options={[
                { value: OverlayMode.Locked, label: t('overlay.modeLocked') },
                { value: OverlayMode.Edit, label: t('overlay.modeEdit') },
              ]}
              onChange={(v) => void overlaySetMode(v)}
            />
          }
        />
        <SettingsRow
          icon="check"
          title={t('overlay.alwaysOnTop')}
          unavailableReason={isMacOS ? t('overlay.alwaysOnTopMac') : undefined}
          control={<span className="text-[var(--color-ink-2)] text-xs">{t('overlay.enabled')}</span>}
        />
      </SettingsGroup>

      <SettingsGroup title={t('overlay.shortcutsGroup')} footer={t('overlay.shortcutsFooter')}>
        {hotkeys.length === 0 ? (
          <SettingsRow title={t('common.loading')} />
        ) : (
          hotkeys.map((h) => (
            <SettingsRow
              key={h.id}
              icon="keyboard"
              title={hotkeyLabel(h.id, t)}
              description={
                h.registered ? (
                  h.customized ? (
                    t('overlay.customized')
                  ) : undefined
                ) : (
                  <span className="text-[var(--color-warn)]">{t('overlay.registerFailed')}</span>
                )
              }
              control={<ShortcutRecorder pretty={h.acceleratorPretty} onRecord={(accel) => void rebind(h.id, accel)} />}
            />
          ))
        )}
      </SettingsGroup>

      <SettingsGroup title={t('overlay.appearanceGroup')} footer={t('overlay.appearanceFooter')}>
        <SettingsRow
          icon="waveform"
          title={t('overlay.fontSize')}
          control={
            <div className="w-56">
              <Slider
                label={t('overlay.fontSize')}
                value={draft.fontSize}
                min={MIN_FONT_SIZE}
                max={MAX_FONT_SIZE}
                format={(v) => `${v} px`}
                onChange={(v) => setDraft((d) => (d ? { ...d, fontSize: v } : d))}
                onCommit={(v) => void updateOverlay({ fontSize: v })}
              />
            </div>
          }
        />
        <SettingsRow
          icon="overlay"
          title={t('overlay.opacity')}
          description={t('overlay.opacityDescription')}
          control={
            <div className="w-56">
              <Slider
                label={t('overlay.opacity')}
                value={draft.opacity}
                min={0}
                max={100}
                format={(v) => `${v}%`}
                onChange={(v) => setDraft((d) => (d ? { ...d, opacity: v } : d))}
                onCommit={(v) => void updateOverlay({ opacity: v })}
              />
            </div>
          }
        />
        <SettingsRow
          icon="translate"
          title={t('overlay.maxLines')}
          description={t('overlay.maxLinesDescription')}
          control={
            <div className="w-56">
              <Slider
                label={t('overlay.maxLines')}
                value={draft.maxLines}
                min={1}
                max={MAX_OVERLAY_LINES}
                format={(v) => t('common.lines', { count: v })}
                onChange={(v) => setDraft((d) => (d ? { ...d, maxLines: v } : d))}
                onCommit={(v) => void updateOverlay({ maxLines: v })}
              />
            </div>
          }
        />
        <SettingsRow
          icon="brush"
          title={t('overlay.fontColor')}
          control={<ColorField value={overlayCfg.fontColor} onCommit={(v) => void updateOverlay({ fontColor: v })} />}
        />
        <SettingsRow
          icon="swatch"
          title={t('overlay.bgColor')}
          control={<ColorField value={overlayCfg.bgColor} onCommit={(v) => void updateOverlay({ bgColor: v })} />}
        />
        <SettingsRow
          icon="sparkle"
          title={t('overlay.glow')}
          control={
            <Switch
              label={t('overlay.glow')}
              checked={overlayCfg.glow}
              onChange={(v) => void updateOverlay({ glow: v })}
            />
          }
        />
        <SettingsRow
          icon="flow"
          title={t('overlay.glowSpeed')}
          unavailableReason={overlayCfg.glow ? undefined : t('overlay.glowSpeedNeedsGlow')}
          control={
            <div className="w-56">
              <Slider
                label={t('overlay.glowSpeed')}
                value={draft.glowSpeed}
                min={0}
                max={MAX_GLOW_SPEED}
                disabled={!overlayCfg.glow}
                format={(v) => `${v}%`}
                onChange={(v) => setDraft((d) => (d ? { ...d, glowSpeed: v } : d))}
                onCommit={(v) => void updateOverlay({ glowSpeed: v })}
              />
            </div>
          }
        />
      </SettingsGroup>
    </Page>
  )
}

const COLOR_COMMIT_DELAY_MS = 150

function ColorField({ value, onCommit }: { value: string; onCommit: (v: string) => void }) {
  const { t } = useI18n()
  const [local, setLocal] = useState(value)
  const timer = useRef<number | null>(null)
  const latest = useRef(value)
  const commit = useRef(onCommit)

  useEffect(() => {
    commit.current = onCommit
  })

  useEffect(() => {
    if (timer.current === null) setLocal(value)
  }, [value])

  useEffect(
    () => () => {
      if (timer.current !== null) {
        window.clearTimeout(timer.current)
        commit.current(latest.current)
      }
    },
    [],
  )

  return (
    <div className="flex items-center gap-2">
      <input
        type="color"
        aria-label={t('overlay.color')}
        value={local}
        onChange={(e) => {
          const next = e.target.value
          latest.current = next
          setLocal(next)
          if (timer.current !== null) window.clearTimeout(timer.current)
          timer.current = window.setTimeout(() => {
            timer.current = null
            commit.current(latest.current)
          }, COLOR_COMMIT_DELAY_MS)
        }}
        className="h-6 w-12 cursor-pointer rounded-[var(--radius-ctl)] border border-[var(--color-border-subtle)] bg-transparent p-0"
      />
      <span className="font-mono text-[var(--color-ink-2)] text-xs">{local}</span>
    </div>
  )
}
