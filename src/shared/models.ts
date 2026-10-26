import type { MessageKey } from './i18n'

const TITLE_KEYS: Record<string, MessageKey> = {
  'asr-builtin': 'models.modelAsrBuiltinTitle',
  'asr-high-accuracy': 'models.modelAsrHighTitle',
  'mt-base': 'models.modelMtBaseTitle',
  'mt-big': 'models.modelMtBigTitle',
}

export function modelTitle(model: { id: string; title: string }, t: (key: MessageKey) => string): string {
  const key = TITLE_KEYS[model.id]
  return key ? t(key) : model.title
}
