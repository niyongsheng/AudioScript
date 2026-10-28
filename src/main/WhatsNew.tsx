import { useEffect, useId } from 'react'
import { type MessageKey, useI18n } from '@/shared/i18n'
import { openUrl } from '@/shared/ipc'
import { Button } from '@/ui/Button'
import { Icon, type IconName } from '@/ui/Icon'
import appIcon from './app-icon.png'

export interface WhatsNewFeature {
  icon: IconName
  titleKey: MessageKey
  bodyKey: MessageKey
}

export const SEEN_VERSION_KEY = 'audioscript.seen-version'

const FEATURES_V030: readonly WhatsNewFeature[] = [
  {
    icon: 'waveform',
    titleKey: 'whatsNew.qwen3Title',
    bodyKey: 'whatsNew.qwen3Body',
  },
  {
    icon: 'translate',
    titleKey: 'whatsNew.macTranslateTitle',
    bodyKey: 'whatsNew.macTranslateBody',
  },
  {
    icon: 'sparkle',
    titleKey: 'whatsNew.glowTitle',
    bodyKey: 'whatsNew.glowBody',
  },
]

const WHATS_NEW: Record<string, readonly WhatsNewFeature[]> = {
  '0.3.0': FEATURES_V030,
}

export function whatsNewFor(version: string): readonly WhatsNewFeature[] | undefined {
  return WHATS_NEW[version]
}

interface Props {
  version: string
  name: string
  features: readonly WhatsNewFeature[]
  onClose: () => void
}

export function WhatsNew({ version, name, features, onClose }: Props) {
  const { t } = useI18n()
  const titleId = useId()
  const notesUrl = `https://github.com/niyongsheng/AudioScript/releases/tag/v${version}`

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [onClose])

  return (
    <div
      className="fixed inset-0 z-[80] flex items-center justify-center bg-black/35 p-6 backdrop-blur-[2px]"
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
    >
      <div className="flex max-h-[calc(100vh-3rem)] w-full max-w-[520px] flex-col overflow-y-auto rounded-[var(--radius-panel)] bg-[var(--color-surface-0)] p-10 shadow-[0_24px_80px_rgb(0_0_0/0.35)]">
        <img
          src={appIcon}
          alt=""
          draggable={false}
          className="mx-auto size-20 rounded-[22px] shadow-[0_10px_30px_rgb(0_0_0/0.18)]"
        />

        <header className="mt-8 space-y-1.5">
          <h1 id={titleId} className="font-semibold text-2xl text-[var(--color-ink-0)] tracking-tight">
            <span className="text-[var(--color-accent)]">{t('whatsNew.welcome')}</span> {name}
          </h1>
          <p className="font-mono text-[var(--color-ink-2)] text-xs">{t('whatsNew.version', { version })}</p>
        </header>

        <ul className="mt-8 space-y-6">
          {features.map((feature) => (
            <li key={feature.titleKey} className="flex gap-4">
              <Icon name={feature.icon} size={20} className="mt-0.5 shrink-0 text-[var(--color-accent)]" />
              <div className="min-w-0">
                <p className="font-medium text-[var(--color-ink-0)] text-sm">{t(feature.titleKey)}</p>
                <p className="mt-1 text-[var(--color-ink-2)] text-xs leading-relaxed">{t(feature.bodyKey)}</p>
              </div>
            </li>
          ))}
        </ul>

        <div className="mt-10 flex items-center justify-between gap-3">
          <button
            type="button"
            onClick={() => void openUrl(notesUrl).catch(() => {})}
            className="text-[var(--color-ink-2)] text-xs underline underline-offset-4 transition hover:text-[var(--color-ink-1)]"
          >
            {t('whatsNew.viewNotes')}
          </button>
          <Button variant="primary" onClick={onClose} className="rounded-full px-5">
            {t('whatsNew.continue')}
          </Button>
        </div>
      </div>
    </div>
  )
}
