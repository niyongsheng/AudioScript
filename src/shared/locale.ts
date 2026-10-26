import { isLocale, setLocale } from './i18n'
import { EVENT, localeGet, on } from './ipc'

let started: Promise<void> | null = null

async function start() {
  try {
    const locale = await localeGet()
    if (isLocale(locale)) setLocale(locale)
  } catch {}

  await on<string>(EVENT.locale, (locale) => {
    if (isLocale(locale)) setLocale(locale)
  })
}

export function initLocale(): Promise<void> {
  if (!started) started = start()
  return started
}
