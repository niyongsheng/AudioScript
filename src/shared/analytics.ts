import { trackEvent } from '@aptabase/tauri'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { call, prefsGet } from '@/shared/ipc'
import { AnalyticsTier } from '@/shared/types'

type Props = Record<string, string | number>

const EVENT_TIER: Record<string, AnalyticsTier> = {
  session_start_failed: AnalyticsTier.Minimal,
  onboarding_completed: AnalyticsTier.Standard,
  update_checked: AnalyticsTier.Standard,
  update_prompt: AnalyticsTier.Standard,
  update_install: AnalyticsTier.Standard,
  permission_result: AnalyticsTier.Standard,
  setting_changed: AnalyticsTier.Standard,
  permission_settings_opened: AnalyticsTier.Full,
  easter_egg_opened: AnalyticsTier.Full,
}

const BUDGETS: Record<AnalyticsTier, number> = {
  [AnalyticsTier.UltraMinimal]: 0,
  [AnalyticsTier.Minimal]: 20,
  [AnalyticsTier.Standard]: 80,
  [AnalyticsTier.Full]: 500,
}

const MAX_MESSAGE = 2_000
const MAX_STACK = 20_000
const MAX_REPORTED = 50

let enabled = false
let tier: AnalyticsTier = AnalyticsTier.UltraMinimal
let sent = 0
const seen = new Set<string>()
const reported = new Set<string>()

export function setTrackingEnabled(next: boolean) {
  enabled = next
  if (!next) {
    reported.clear()
    return
  }
  sent = 0
  seen.clear()
}

export function setTrackingTier(next: AnalyticsTier) {
  tier = next
}

export function track(name: string, props?: Props) {
  if (!enabled || tier === AnalyticsTier.UltraMinimal) return
  const minTier = EVENT_TIER[name] ?? AnalyticsTier.Full
  if (tier === AnalyticsTier.Minimal && minTier !== AnalyticsTier.Minimal) return
  if (tier === AnalyticsTier.Standard && minTier === AnalyticsTier.Full) return
  if (tier === AnalyticsTier.Standard) {
    const key = `${name}:${JSON.stringify(props ?? {})}`
    if (seen.has(key)) return
    seen.add(key)
  }
  if (sent >= BUDGETS[tier]) return
  sent += 1
  trackEvent(name, props).catch(() => {})
}

export function syncTrackingFromPrefs() {
  try {
    prefsGet()
      .then((p) => setTrackingEnabled(p.analyticsEnabled))
      .catch(() => {})
  } catch {
    // 浏览器预览没有 Tauri IPC。
  }
}

function describe(error: unknown): { name: string; message: string; stack?: string } {
  if (error instanceof Error) {
    return { name: error.name || 'Error', message: error.message, stack: error.stack }
  }
  if (typeof error === 'string') return { name: 'Error', message: error }
  try {
    const json = JSON.stringify(error)
    if (json) return { name: 'Error', message: json }
  } catch {
    // 循环引用等无法序列化的 rejection 退回到固定文案。
  }
  return { name: 'Error', message: 'Unserializable error' }
}

function currentWindowLabel(): string | undefined {
  try {
    return getCurrentWindow().label
  } catch {
    return undefined
  }
}

function reportError(
  error: unknown,
  kind: 'error' | 'unhandledrejection',
  extra: { source?: string; line?: number; column?: number } = {},
) {
  if (!enabled) return
  const { name, message, stack } = describe(error)
  const report = {
    kind,
    name: name.slice(0, 120),
    message: (message || 'Unknown error').slice(0, MAX_MESSAGE),
    stack: stack?.slice(0, MAX_STACK),
    window: currentWindowLabel(),
    ...extra,
  }
  const key = `${kind}:${report.name}:${report.message}:${report.stack?.slice(0, 200) ?? ''}`
  if (reported.has(key) || reported.size >= MAX_REPORTED) return
  reported.add(key)
  try {
    call('frontend_error', { report }).catch(() => {})
  } catch {
    // 浏览器预览没有 Tauri IPC。
  }
}

if (typeof window !== 'undefined') {
  window.addEventListener('error', (event) => {
    if (!event.error) {
      const message = String(event.message || '')
      if (!message || message === 'Script error.' || message.includes('ResizeObserver loop')) return
    }
    reportError(event.error ?? event.message, 'error', {
      source: event.filename || undefined,
      line: event.lineno || undefined,
      column: event.colno || undefined,
    })
  })
  window.addEventListener('unhandledrejection', (event) => {
    reportError(event.reason, 'unhandledrejection')
  })
}
