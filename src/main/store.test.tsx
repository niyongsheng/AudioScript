import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AppInfo, Preferences } from '@/shared/ipc'
import { EVENT } from '@/shared/ipc'
import { TaskStatus, Theme } from '@/shared/types'
import { StoreProvider, useStore } from './store'

const ipc = vi.hoisted(() => ({
  appInfo: vi.fn(),
  on: vi.fn(),
  prefsGet: vi.fn(),
  prefsSet: vi.fn(),
  sessionStatus: vi.fn(),
  sessionStart: vi.fn(),
  updateInstall: vi.fn(),
  setTrackingEnabled: vi.fn(),
}))

vi.mock('@/shared/analytics', () => ({
  setTrackingEnabled: ipc.setTrackingEnabled,
  track: vi.fn(),
}))

vi.mock('@/shared/ipc', async () => ({
  ...(await vi.importActual<typeof import('@/shared/ipc')>('@/shared/ipc')),
  appInfo: ipc.appInfo,
  on: ipc.on,
  prefsGet: ipc.prefsGet,
  prefsSet: ipc.prefsSet,
  sessionStatus: ipc.sessionStatus,
  sessionStart: ipc.sessionStart,
  updateInstall: ipc.updateInstall,
  hotkeyFailures: vi.fn(async () => []),
  shortcutsGet: vi.fn(async () => []),
  overlayConfigGet: vi.fn(async () => null),
  overlayIsVisible: vi.fn(async () => false),
  permissionStatus: vi.fn(async () => null),
  audioDevices: vi.fn(async () => []),
}))

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((yes) => {
    resolve = yes
  })
  return { promise, resolve }
}

describe('StoreProvider session and fork capabilities', () => {
  let container: HTMLDivElement
  let root: Root
  let store: ReturnType<typeof useStore>
  let listeners: Map<string, (value: unknown) => void>
  let info: AppInfo

  function Probe() {
    store = useStore()
    return null
  }

  async function render() {
    await act(async () => {
      root.render(
        <StoreProvider>
          <Probe />
        </StoreProvider>,
      )
    })
  }

  beforeEach(() => {
    for (const mock of Object.values(ipc)) mock.mockReset()
    vi.stubGlobal('matchMedia', () => ({
      matches: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }))
    listeners = new Map()
    ipc.on.mockImplementation(async (name: string, handler: (value: unknown) => void) => {
      listeners.set(name, handler)
      return () => listeners.delete(name)
    })
    info = {
      name: 'AudioScript',
      version: '0.2.10',
      platform: 'macos',
      arch: 'aarch64',
      builtAtMs: null,
      analyticsAvailable: false,
      analyticsDisabledReason: '未配置此 fork 的统计 Key',
      updaterEnabled: false,
      updaterDisabledReason: '未配置此 fork 的更新公钥',
      links: {
        repository: 'https://github.com/zyg0733/AudioScript',
        site: 'https://zyg0733.github.io/AudioScript/',
        upstreamRepository: 'https://github.com/niyongsheng/AudioScript',
        upstreamSponsor: 'https://ifdian.net/a/nico2026',
        updaterEndpoint: 'https://github.com/zyg0733/AudioScript/releases/latest/download/latest.json',
        modelsRelease: 'https://github.com/zyg0733/AudioScript/releases/download/models-v1',
      },
    }
    ipc.appInfo.mockImplementation(async () => info)
    ipc.prefsGet.mockResolvedValue({ theme: Theme.Light, analyticsEnabled: false } as Preferences)
    ipc.sessionStatus.mockResolvedValue({ status: TaskStatus.Ready, device: null, error: null })
    container = document.createElement('div')
    document.body.append(container)
    root = createRoot(container)
  })

  afterEach(async () => {
    await act(async () => root.unmount())
    container.remove()
    vi.unstubAllGlobals()
  })

  it('restores a running session and its active device when the window mounts', async () => {
    ipc.sessionStatus.mockResolvedValue({ status: TaskStatus.Capturing, device: '系统声音', error: null })
    await render()
    expect(store.status).toBe(TaskStatus.Capturing)
    expect(store.activeDevice).toBe('系统声音')
  })

  it('refreshes the session snapshot after a tray start and clears it after stop', async () => {
    await render()
    ipc.sessionStatus.mockResolvedValue({ status: TaskStatus.Capturing, device: '麦克风', error: null })
    await act(async () => listeners.get(EVENT.taskState)?.(TaskStatus.Capturing))
    expect(store.activeDevice).toBe('麦克风')

    ipc.sessionStatus.mockResolvedValue({ status: TaskStatus.Ready, device: null, error: null })
    await act(async () => listeners.get(EVENT.taskState)?.(TaskStatus.Ready))
    expect(store.status).toBe(TaskStatus.Ready)
    expect(store.activeDevice).toBeNull()
  })

  it('shows runtime faults and rejects an older pending session snapshot', async () => {
    const pending = deferred<{ status: TaskStatus; device: string; error: null }>()
    ipc.sessionStatus.mockReturnValue(pending.promise)
    await render()
    await act(async () => listeners.get(EVENT.sessionError)?.('设备已断开，请重新选择音源'))
    await act(async () => pending.resolve({ status: TaskStatus.Capturing, device: '旧设备', error: null }))
    expect(store.status).toBe(TaskStatus.Error)
    expect(store.activeDevice).toBeNull()
    expect(store.error).toBe('设备已断开，请重新选择音源')
  })

  it('restores the backend fault reason when reopening the window', async () => {
    ipc.sessionStatus.mockResolvedValue({ status: TaskStatus.Error, device: null, error: '推理失败' })
    await render()
    expect(store.status).toBe(TaskStatus.Error)
    expect(store.error).toBe('推理失败')
  })

  it('does not let a start response overwrite an early runtime failure', async () => {
    await render()
    const start = deferred<TaskStatus>()
    ipc.sessionStart.mockReturnValue(start.promise)
    let toggle!: Promise<void>
    await act(async () => {
      toggle = store.toggleSession()
    })
    ipc.sessionStatus.mockResolvedValue({ status: TaskStatus.Error, device: null, error: '采集启动后断开' })
    await act(async () => {
      listeners.get(EVENT.sessionError)?.('采集启动后断开')
      start.resolve(TaskStatus.Capturing)
      await toggle
    })
    expect(store.status).toBe(TaskStatus.Error)
    expect(store.error).toBe('采集启动后断开')
    expect(store.activeDevice).toBeNull()
  })

  it('does not enable tracking from saved preferences when this fork has no key', async () => {
    ipc.prefsGet.mockResolvedValue({ theme: Theme.Light, analyticsEnabled: true } as Preferences)
    await render()
    expect(ipc.setTrackingEnabled).not.toHaveBeenCalledWith(true)
    await act(async () => store.setAnalyticsEnabled(true))
    expect(ipc.prefsSet).not.toHaveBeenCalled()
    expect(store.error).toBe(info.analyticsDisabledReason)
  })

  it('keeps tracking disabled when saving opt-in fails', async () => {
    info.analyticsAvailable = true
    ipc.prefsSet.mockRejectedValue(new Error('写入失败'))
    await render()
    await act(async () => store.setAnalyticsEnabled(true))
    expect(store.analyticsEnabled).toBe(false)
    expect(ipc.setTrackingEnabled).not.toHaveBeenCalledWith(true)
    expect(store.error).toContain('写入失败')
  })

  it('rejects update installation when this build has no trusted fork public key', async () => {
    await render()
    await act(async () => store.installUpdate())
    expect(ipc.updateInstall).not.toHaveBeenCalled()
    expect(store.updateError).toBe(info.updaterDisabledReason)
  })

  it('unsubscribes all session and window events on unmount', async () => {
    await render()
    expect(listeners.has(EVENT.sessionError)).toBe(true)
    await act(async () => root.unmount())
    expect(listeners.size).toBe(0)
    root = createRoot(container)
  })
})
