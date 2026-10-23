import { useEffect, useState } from 'react'
import { useStore } from '@/main/store'
import { track } from '@/shared/analytics'
import { autostartGet, autostartSet, formatCmdError, logsExport, prefsReset, revealPath, Theme } from '@/shared/ipc'
import { Alert } from '@/ui/Alert'
import { Button } from '@/ui/Button'
import { Page } from '@/ui/Page'
import { Select } from '@/ui/Select'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'
import { Switch } from '@/ui/Switch'

const THEME_OPTIONS = [
  { value: Theme.System, label: '跟随系统' },
  { value: Theme.Light, label: '浅色' },
  { value: Theme.Dark, label: '深色' },
]

export function GeneralPage() {
  const { info, theme, changeTheme, prefsReady, analyticsEnabled, setAnalyticsEnabled, setError } = useStore()

  const [autostart, setAutostart] = useState<boolean | null>(null)
  const [confirmingReset, setConfirmingReset] = useState(false)
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<string | null>(null)

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
      setNote('已恢复默认设置。部分改动需重启应用生效。')
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
      setNote(`日志已导出到 ${path}`)
    } catch (e) {
      setError(formatCmdError(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Page title="通用">
      {note && <Alert tone="info">{note}</Alert>}

      <SettingsGroup title="外观">
        <SettingsRow
          icon="settings"
          title="主题"
          description="悬浮窗始终为深色。"
          control={<Select label="主题" value={theme} options={THEME_OPTIONS} onChange={(v) => void changeTheme(v)} />}
        />
      </SettingsGroup>

      <SettingsGroup title="启动">
        <SettingsRow
          icon="play"
          title="登录时启动"
          description="关闭主窗口后应用继续驻留托盘。"
          control={
            <Switch
              label="登录时启动"
              checked={autostart === true}
              disabled={autostart === null}
              onChange={(v) => void toggleAutostart(v)}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title="隐私" footer="统计仅包含事件名与少量枚举属性，不含转写文本、设备名称或文件路径。">
        <SettingsRow
          icon="shield"
          title="匿名使用统计"
          description={
            info?.analyticsAvailable
              ? '默认关闭。开启后通过此 fork 配置的 Aptabase 发送功能使用情况。'
              : (info?.analyticsDisabledReason ?? '正在读取统计配置…')
          }
          control={
            <Switch
              label="匿名使用统计"
              checked={analyticsEnabled && info?.analyticsAvailable === true}
              disabled={!prefsReady || !info?.analyticsAvailable}
              onChange={setAnalyticsEnabled}
            />
          }
        />
      </SettingsGroup>

      <SettingsGroup title="诊断">
        <SettingsRow
          icon="folder"
          title="导出日志"
          description="复制到桌面并以日期命名，随后自动打开。"
          control={
            <Button icon="external" size="sm" disabled={busy} onClick={() => void doExportLogs()}>
              导出
            </Button>
          }
        />
        <SettingsRow
          icon="folder"
          title="打开日志文件夹"
          description="直接定位到原始日志目录。"
          onClick={() => void revealPath().catch((e) => setError(formatCmdError(e)))}
        />
      </SettingsGroup>

      <SettingsGroup title="设置管理">
        <SettingsRow
          icon="reset"
          title="重置所有设置"
          description="清除全部设置，不可撤销。下次启动会重新显示欢迎页。"
          control={
            confirmingReset ? (
              <div className="flex gap-1.5">
                <Button variant="danger" size="sm" disabled={busy} onClick={() => void doReset()}>
                  确认重置
                </Button>
                <Button size="sm" onClick={() => setConfirmingReset(false)}>
                  取消
                </Button>
              </div>
            ) : (
              <Button variant="danger" size="sm" onClick={() => setConfirmingReset(true)}>
                重置
              </Button>
            )
          }
        />
      </SettingsGroup>
    </Page>
  )
}
