import { useEffect, useState } from 'react'
import { useStore } from '@/main/store'
import { formatCmdError, type ModelsList, modelsList, openRepo, openSponsor, openUrl } from '@/shared/ipc'
import { ModelKind, TaskStatus } from '@/shared/types'
import { Button } from '@/ui/Button'
import { Icon } from '@/ui/Icon'
import { Page } from '@/ui/Page'
import { ProgressBar } from '@/ui/ProgressBar'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'

export function AboutPage() {
  const {
    info,
    setError,
    status,
    updateInfo,
    updatePhase,
    updateProgress,
    updateError,
    installedVersion,
    checkUpdate,
    installUpdate,
    restartApp,
  } = useStore()
  const [models, setModels] = useState<ModelsList | null>(null)
  const [modelsFailed, setModelsFailed] = useState(false)
  const [licensesOpen, setLicensesOpen] = useState(false)

  useEffect(() => {
    let disposed = false
    modelsList()
      .then((m) => {
        if (!disposed) setModels(m)
      })
      .catch(() => {
        if (!disposed) setModelsFailed(true)
      })
    return () => {
      disposed = true
    }
  }, [])

  async function openSite() {
    if (!info) return
    try {
      await openUrl(info.links.site)
    } catch (e) {
      setError(formatCmdError(e))
    }
  }

  async function openSponsorPage() {
    try {
      await openSponsor()
    } catch (e) {
      setError(formatCmdError(e))
    }
  }

  const capturing = status === TaskStatus.Capturing
  const updaterEnabled = info?.updaterEnabled === true && updateInfo?.enabled !== false
  const percent =
    updateProgress?.total != null
      ? Math.min(100, Math.floor((updateProgress.downloaded / updateProgress.total) * 100))
      : null

  const updateDescription = (() => {
    if (info && !updaterEnabled) {
      return updateInfo?.disabledReason ?? info.updaterDisabledReason ?? '自动更新已禁用'
    }
    if (updatePhase === 'downloading') {
      return `正在下载并安装 v${updateInfo?.version ?? ''}${percent == null ? '' : `（${percent}%）`}…`
    }
    if (updatePhase === 'ready') {
      const version = installedVersion ?? updateInfo?.version ?? ''
      return capturing ? `已安装 v${version}，请先停止转写再重启` : `已安装 v${version}，重启应用后生效`
    }
    if (updatePhase === 'error') return `更新失败：${updateError ?? '未知错误'}`
    if (updatePhase === 'checking') return '正在检查最新版本…'
    if (updateInfo?.available) {
      return capturing ? '发现新版本，请先停止转写再更新' : `发现新版本 v${updateInfo.version}`
    }
    if (updateInfo) return `已是最新版本（v${updateInfo.currentVersion}）`
    return '启动时自动检查，也可手动检查'
  })()

  return (
    <Page title="关于">
      <SettingsGroup title="应用程序">
        <SettingsRow icon="info" title="版本" control={<Mono>{info?.version ?? '—'}</Mono>} />
        <SettingsRow
          icon="download"
          title="检查更新"
          description={
            <>
              {updateDescription}
              {updateInfo?.available && updatePhase === 'idle' && updateInfo.notes && (
                <p className="mt-1 line-clamp-3 whitespace-pre-line">{updateInfo.notes}</p>
              )}
            </>
          }
          control={
            updatePhase === 'downloading' ? (
              <div className="flex items-center gap-2">
                <ProgressBar value={percent} label="更新下载进度" />
                <Mono>{percent == null ? '下载中' : `${percent}%`}</Mono>
              </div>
            ) : updatePhase === 'ready' ? (
              <Button size="sm" variant="primary" icon="refresh" onClick={() => void restartApp()} disabled={capturing}>
                重启
              </Button>
            ) : updateInfo?.available ? (
              <Button
                size="sm"
                variant="primary"
                icon="download"
                onClick={() => void installUpdate()}
                disabled={!updaterEnabled || capturing || updatePhase === 'checking'}
              >
                下载并安装
              </Button>
            ) : (
              <Button
                size="sm"
                icon="refresh"
                onClick={() => void checkUpdate()}
                disabled={!updaterEnabled || updatePhase === 'checking'}
                pending={updatePhase === 'checking'}
              >
                检查
              </Button>
            )
          }
        />
        <SettingsRow
          icon="chip"
          title="平台"
          control={
            <Mono>
              {info?.platform ?? '—'} · {info?.arch ?? '—'}
            </Mono>
          }
        />
        <SettingsRow
          icon="external"
          title="官网"
          onClick={() => void openSite()}
          control={<Icon name="chevron" size={16} className="mt-0.5 text-[var(--color-ink-2)]" />}
        />
      </SettingsGroup>

      <SettingsGroup title="原项目与作者">
        <SettingsRow
          icon="github"
          title="基于 Nico 的 AudioScript 继续开发"
          description="当前分支由 zyg0733 维护，保留原作者署名与 MIT 许可。"
          onClick={() => info && void openUrl(info.links.upstreamRepository).catch((e) => setError(formatCmdError(e)))}
          control={<Icon name="chevron" size={16} className="mt-0.5 text-[var(--color-ink-2)]" />}
        />
        <SettingsRow
          icon="heart"
          title="赞助原作者 Nico"
          description="爱发电入口属于原作者，款项用于支持原作者。"
          onClick={() => void openSponsorPage()}
          control={<Icon name="chevron" size={16} className="mt-0.5 text-[var(--color-ink-2)]" />}
        />
      </SettingsGroup>

      <SettingsGroup title="开源许可">
        <SettingsRow
          icon="github"
          title={
            <span className="flex items-center gap-2">
              LICENSE
              <Mono>MIT</Mono>
            </span>
          }
          onClick={() => void openRepo()}
          control={<Icon name="chevron" size={16} className="mt-0.5 text-[var(--color-ink-2)]" />}
        />
      </SettingsGroup>

      <SettingsGroup
        title="模型与许可"
        footer={
          licensesOpen
            ? '本项目把 OPUS-MT 权重转换为 CTranslate2 int8，并把 SentencePiece 词表转换为 tokenizer JSON；未更改训练数据。'
            : undefined
        }
      >
        <SettingsRow
          icon="shield"
          title="模型许可"
          onClick={() => setLicensesOpen((open) => !open)}
          control={
            <Icon
              name="chevron"
              size={16}
              className={`mt-0.5 text-[var(--color-ink-2)] transition-transform ${licensesOpen ? 'rotate-90' : ''}`}
            />
          }
        />
        {licensesOpen &&
          (models ? (
            [...models.asr, ...models.mt].flatMap((m) =>
              m.licenses.map((l) => (
                <SettingsRow
                  key={`${m.id}-${l.component}`}
                  icon={m.kind === ModelKind.Asr ? 'waveform' : 'translate'}
                  title={l.component}
                  description={
                    <>
                      {m.title} · {l.license}
                      {l.requiresAttribution && ' · 需署名'}
                      {l.attribution && ` · ${l.attribution}`}
                    </>
                  }
                  control={
                    l.licenseUrl ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        icon="external"
                        onClick={() => void openUrl(l.licenseUrl as string).catch((e) => setError(formatCmdError(e)))}
                      >
                        许可
                      </Button>
                    ) : undefined
                  }
                />
              )),
            )
          ) : (
            <SettingsRow title={modelsFailed ? '模型许可读取失败，请到「模型」页重试' : '读取中…'} />
          ))}
      </SettingsGroup>
    </Page>
  )
}

function Mono({ children }: { children: React.ReactNode }) {
  return <span className="font-mono text-[var(--color-ink-1)] text-xs">{children}</span>
}
