import { trackError, trackEvent } from '@aptabase/tauri'

type Props = Record<string, string | number>

let enabled = false

export function setTrackingEnabled(next: boolean) {
  enabled = next
}

export function track(name: string, props?: Props) {
  if (!enabled) return
  trackEvent(name, props).catch(() => {})
}

export function reportError(error: unknown) {
  if (!enabled) return
  trackError(error).catch(() => {})
}

if (typeof window !== 'undefined') {
  window.addEventListener('error', (event) => {
    if (!event.error) {
      const message = String(event.message || '')
      if (!message || message === 'Script error.' || message.includes('ResizeObserver loop')) return
    }
    reportError(event.error ?? event.message)
  })
  window.addEventListener('unhandledrejection', (event) => {
    reportError(event.reason)
  })
}
