import type { MessageKey } from './i18n'

const LABEL_KEYS: Record<string, MessageKey> = {
  'overlay.toggleMode': 'overlay.shortcutToggleMode',
  'overlay.toggleVisible': 'overlay.shortcutToggleVisible',
}

export function hotkeyLabel(id: string, t: (key: MessageKey) => string): string {
  const key = LABEL_KEYS[id]
  return key ? t(key) : id
}
