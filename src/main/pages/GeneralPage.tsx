import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { PermissionPanel } from '@/main/permissions/PermissionPanel'
import { useStore } from '@/main/store'
import { track } from '@/shared/analytics'
import { useI18n } from '@/shared/i18n'
import {
  autostartGet,
  autostartSet,
  formatCmdError,
  Language,
  logsExport,
  prefsReset,
  revealPath,
  Theme,
} from '@/shared/ipc'
import { Alert } from '@/ui/Alert'
import { Button } from '@/ui/Button'
import { Page } from '@/ui/Page'
import { Select } from '@/ui/Select'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'
import { Switch } from '@/ui/Switch'

interface Props {
  focusPermissions: boolean
  onFocusPermissionsHandled: () => void
}

export function GeneralPage({ focusPermissions, onFocusPermissionsHandled }: Props) {
  const { theme, changeTheme, prefsReady, analyticsEnabled, setAnalyticsEnabled, language, changeLanguage, setError } =
    useStore()
  const { t } = useI18n()

  const [autostart, setAutostart] = useState<boolean | null>(null)
  const [confirmingReset, setConfirmingReset] = useState(false)
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<string | null>(null)
  const permissionsRef = useRef<HTMLDivElement>(null)

  useLayoutEffect(() => {
    if (!focusPermissions) return
    permissionsRef.current?.scrollIntoView({ block: 'start' })
    onFocusPermissionsHandled()
  }, [focusPermissions, onFocusPermissionsHandled])

  const themeOptions = [
    { value: Theme.System, label: t('general.themeSystem') },
    { value: Theme.Light, label: t('general.themeLight') },
    { value: Theme.Dark, label: t('general.themeDark') },
  ]

  const languageOptions = [
    { value: Language.System, label: t('language.system') },
    { value: Language.ZhCn, label: t('language.zhCn') },
    { value: Language.ZhTw, label: t('language.zhTw') },
    { value: Language.En, label: t('language.en') },
    { value: Language.Ja, label: t('language.ja') },
  ]

  useEffect(() => {
    autostartGet()
      .then((s) => setAutostart(s.enabled))
      .catch(() => setAutostart(null))
  }, [])

  async function toggleAutostart(next: boolean) {
    try {
      const s = await autostartSet(next)
      setAutostart(s.enabled)
      track('setting_changed', { key: 'autostart', value: s.enabled ? 'on' : 'off' })
    } catch (e) {
      setError(formatCmdError(e))
    }
  }

  async function doReset() {
    setBusy(true)
    setNote(null)
    try {
      await prefsReset()
      setNote(t('general.resetDone'))
    } catch (e) {
      setError(formatCmdError(e))
    } finally {
      setBusy(false)
      setConfirmingReset(false)
    }
  }

  async function doExportLogs() {
    setBusy(true)
    setNote(null)
    try {
      const { path } = await logsExport()
      await revealPath(path)
      setNote(t('general.logsExported', { path }))
    } catch (e) {
      setError(formatCmdError(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Page title={t('general.title')}>
      {note && (
        <Alert tone="info" onDismiss={() => setNote(null)}>
          {note}
        </Alert>
      )}

      <SettingsGroup title={t('general.appearance')}>
        <SettingsRow
          icon="settings"
          title={t('general.theme')}
          description={t('general.themeDescription')}
          control={
            <Select
              label={t('general.theme')}
              value={theme}
              options={themeOptions}
              onChange={(v) => void changeTheme(v)}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t('language.title')}>
        <SettingsRow
          icon="translate"
          title={t('language.title')}
          description={t('language.description')}
          control={
            <Select
              label={t('language.title')}
              value={language}
              options={languageOptions}
              onChange={(v) => void changeLanguage(v)}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t('general.startup')}>
        <SettingsRow
          icon="play"
          title={t('general.autostart')}
          description={t('general.autostartDescription')}
          control={
            <Switch
              label={t('general.autostart')}
              checked={autostart === true}
              disabled={autostart === null}
              onChange={(v) => void toggleAutostart(v)}
            />
          }
        />
      </SettingsGroup>

      <div ref={permissionsRef} className="scroll-mt-6">
        <PermissionPanel />
      </div>

      <SettingsGroup title={t('general.privacy')} footer={t('general.privacyFooter')}>
        <SettingsRow
          icon="shield"
          title={t('general.analytics')}
          description={t('general.analyticsDescription')}
          control={
            <Switch
              label={t('general.analytics')}
              checked={analyticsEnabled}
              disabled={!prefsReady}
              onChange={setAnalyticsEnabled}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title={t('general.diagnostics')}>
        <SettingsRow
          icon="folder"
          title={t('general.exportLogs')}
          description={t('general.exportLogsDescription')}
          control={
            <Button icon="external" size="sm" disabled={busy} onClick={() => void doExportLogs()}>
              {t('common.export')}
            </Button>
          }
        />
        <SettingsRow
          icon="folder"
          title={t('general.openLogs')}
          description={t('general.openLogsDescription')}
          onClick={() => void revealPath().catch((e) => setError(formatCmdError(e)))}
        />
      </SettingsGroup>

      <SettingsGroup title={t('general.manage')}>
        <SettingsRow
          icon="reset"
          title={t('general.resetAll')}
          description={t('general.resetAllDescription')}
          control={
            confirmingReset ? (
              <div className="flex gap-1.5">
                <Button variant="danger" size="sm" disabled={busy} onClick={() => void doReset()}>
                  {t('common.confirmReset')}
                </Button>
                <Button size="sm" onClick={() => setConfirmingReset(false)}>
                  {t('common.cancel')}
                </Button>
              </div>
            ) : (
              <Button variant="danger" size="sm" onClick={() => setConfirmingReset(true)}>
                {t('common.reset')}
              </Button>
            )
          }
        />
      </SettingsGroup>
    </Page>
  )
}
