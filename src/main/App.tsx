import { useCallback, useEffect, useMemo, useState } from 'react'
import { hotkeyLabel } from '@/shared/hotkeys'
import { useI18n } from '@/shared/i18n'
import { openSystemSettings } from '@/shared/ipc'
import { TaskStatus } from '@/shared/types'
import { Alert } from '@/ui/Alert'
import { Button } from '@/ui/Button'
import { LevelStatus } from '@/ui/LevelStatus'
import { PrimaryAction } from '@/ui/PrimaryAction'
import { Sidebar } from '@/ui/Sidebar'
import { TitleBarSpacer } from '@/ui/TitleBarSpacer'
import { Brand } from './Brand'
import { navItems, type PageId } from './navigation'
import { AboutPage } from './pages/AboutPage'
import { AudioPage } from './pages/AudioPage'
import { GeneralPage } from './pages/GeneralPage'
import { ModelsPage } from './pages/ModelsPage'
import { OverlayPage } from './pages/OverlayPage'
import { TranscribePage } from './pages/TranscribePage'
import { TranslatePage } from './pages/TranslatePage'
import { StoreProvider, useStore } from './store'
import { UpdateToast } from './UpdateToast'
import { WelcomeGate } from './WelcomeGate'
import { SEEN_VERSION_KEY, WhatsNew, type WhatsNewFeature, whatsNewFor } from './WhatsNew'

export function App() {
  return (
    <StoreProvider>
      <Root />
    </StoreProvider>
  )
}

function Root() {
  const { info, prefsReady, onboardingDone } = useStore()
  const [whatsNew, setWhatsNew] = useState<readonly WhatsNewFeature[] | null>(null)

  const closeWhatsNew = useCallback(() => {
    if (!info) return
    localStorage.setItem(SEEN_VERSION_KEY, info.version)
    setWhatsNew(null)
  }, [info])

  useEffect(() => {
    if (!prefsReady || !info) return

    const seen = localStorage.getItem(SEEN_VERSION_KEY)
    if (seen === info.version) return

    const features = whatsNewFor(info.version)
    if (!seen && !onboardingDone) {
      localStorage.setItem(SEEN_VERSION_KEY, info.version)
      return
    }
    if (features) {
      setWhatsNew(features)
    } else {
      localStorage.setItem(SEEN_VERSION_KEY, info.version)
    }
  }, [prefsReady, info, onboardingDone])

  if (!prefsReady) return <BootSplash />
  if (!onboardingDone) return <WelcomeGate />

  return (
    <>
      <Shell />
      {whatsNew && info && (
        <WhatsNew version={info.version} name={info.name} features={whatsNew} onClose={closeWhatsNew} />
      )}
    </>
  )
}

function BootSplash() {
  return (
    <main className="flex h-full">
      <section className="flex min-w-0 flex-1 flex-col bg-[var(--color-surface-0)]">
        <TitleBarSpacer />
      </section>
    </main>
  )
}

function Shell() {
  const [page, setPage] = useState<PageId>('transcribe')
  const [focusPermissions, setFocusPermissions] = useState(false)
  const { status } = useStore()
  const { locale } = useI18n()
  // biome-ignore lint/correctness/useExhaustiveDependencies: t() 读取模块级 locale，需要随 locale 重算
  const items = useMemo(() => navItems(), [locale])
  const capturing = status === TaskStatus.Capturing

  const openPermissions = useCallback(() => {
    setPage('general')
    setFocusPermissions(true)
  }, [])
  const handlePermissionsFocused = useCallback(() => setFocusPermissions(false), [])

  return (
    <main className="flex h-full">
      <Sidebar items={items} current={page} onNavigate={setPage} brand={<Brand />} header={<PrimaryActionSlot />} />

      <section className="flex min-w-0 flex-1 flex-col bg-[var(--color-surface-0)]">
        <TitleBarSpacer />

        <Alerts onOpenPermissions={openPermissions} onOpenTranslate={() => setPage('translate')} />

        <div className="flex-1 overflow-y-auto">
          <PageSwitch
            page={page}
            onOpenPermissions={openPermissions}
            focusPermissions={focusPermissions}
            onFocusPermissionsHandled={handlePermissionsFocused}
          />
        </div>

        <footer className="border-[var(--color-border-subtle)] border-t py-2">
          <div className="flex items-center px-6">
            <LevelStatus active={capturing} />
          </div>
        </footer>
      </section>

      <UpdateToast hidden={page === 'about'} />
    </main>
  )
}

function PrimaryActionSlot() {
  const { status, pending, activeDevice, toggleSession } = useStore()
  return (
    <PrimaryAction
      capturing={status === TaskStatus.Capturing}
      pending={pending}
      activeDevice={activeDevice}
      onToggle={() => void toggleSession()}
    />
  )
}

interface PageSwitchProps {
  page: PageId
  onOpenPermissions: () => void
  focusPermissions: boolean
  onFocusPermissionsHandled: () => void
}

function PageSwitch({ page, onOpenPermissions, focusPermissions, onFocusPermissionsHandled }: PageSwitchProps) {
  switch (page) {
    case 'transcribe':
      return <TranscribePage />
    case 'audio':
      return <AudioPage onOpenPermissions={onOpenPermissions} />
    case 'translate':
      return <TranslatePage />
    case 'overlay':
      return <OverlayPage />
    case 'general':
      return <GeneralPage focusPermissions={focusPermissions} onFocusPermissionsHandled={onFocusPermissionsHandled} />
    case 'models':
      return <ModelsPage />
    case 'about':
      return <AboutPage />
  }
}

interface AlertsProps {
  onOpenPermissions: () => void
  onOpenTranslate: () => void
}

function Alerts({ onOpenPermissions, onOpenTranslate }: AlertsProps) {
  const { t } = useI18n()
  const {
    error,
    errorSettingsPane,
    setError,
    hotkeyProblems,
    audio,
    languageMismatch,
    setLanguageMismatch,
    status,
    permissionError,
    setPermissionError,
  } = useStore()

  const showMismatch = languageMismatch && status === TaskStatus.Capturing
  const showSilent = Boolean(audio?.silent && audio.segments === 0 && status === TaskStatus.Capturing)
  const hasAlerts =
    Boolean(error) || Boolean(permissionError) || showMismatch || hotkeyProblems.length > 0 || showSilent

  if (!hasAlerts) return null

  return (
    <div className="mx-auto w-full max-w-2xl space-y-2 px-6 pt-4">
      {error && (
        <Alert
          tone="error"
          title={t('app.operationFailed')}
          onDismiss={() => setError(null)}
          actions={
            errorSettingsPane && (
              <Button size="sm" onClick={() => void openSystemSettings(errorSettingsPane).catch(() => {})}>
                {t('translate.languagePackOpen')}
              </Button>
            )
          }
        >
          <p className="break-words font-mono">{error}</p>
        </Alert>
      )}

      {permissionError && (
        <Alert
          tone="warn"
          title={t('app.permissionMissing')}
          onDismiss={() => setPermissionError(null)}
          actions={
            <Button size="sm" onClick={onOpenPermissions}>
              {t('app.permissionGo')}
            </Button>
          }
        >
          <p>{permissionError.message}</p>
        </Alert>
      )}

      {showMismatch && (
        <Alert
          tone="warn"
          title={t('app.languageMismatch')}
          onDismiss={() => setLanguageMismatch(false)}
          actions={
            <Button size="sm" onClick={onOpenTranslate}>
              {t('app.languageMismatchGo')}
            </Button>
          }
        >
          {t('app.languageMismatchDetail')}
        </Alert>
      )}

      {hotkeyProblems.length > 0 && (
        <Alert tone="warn" title={t('app.hotkeyFailed', { count: hotkeyProblems.length })}>
          {hotkeyProblems.map((h) => `${h.accelerator} ${hotkeyLabel(h.id, t)}`).join(t('common.listSeparator'))}
          {t('app.hotkeyFailedDetail')}
        </Alert>
      )}

      {showSilent && (
        <Alert tone="warn" title={t('app.audioSilent')}>
          {t('app.audioSilentDetail')}
        </Alert>
      )}
    </div>
  )
}
