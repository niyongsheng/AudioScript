#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../../../..')
const LOCALES = [
  'zh-CN',
  'zh-TW',
  'en',
  'ja',
  'ko',
  'fr',
  'de',
  'es',
  'pt-BR',
  'ru',
  'it',
  'id',
  'ar',
]
const BASE = 'zh-CN'
const CJK = /[぀-ヿ㐀-䶿一-鿿가-힯]/
const CJK_SCRIPTS = ['en', 'fr', 'de', 'es', 'pt-BR', 'ru', 'it', 'id', 'ar']
const CJK_ALLOWED_KEYS = /^language\./
const CJK_ALLOWED_VALUES = /网易云音乐/
const PLURAL_LOCALES = ['en', 'fr', 'de', 'es', 'pt-BR', 'ru', 'it', 'id']

const args = process.argv.slice(2)
const strict = args.includes('--strict')
const sinceFlag = args.indexOf('--since')
const since =
  sinceFlag !== -1 ? args[sinceFlag + 1] : lastTag()

const errors = []
const warnings = []
const ok = (msg) => process.stdout.write(`✓ ${msg}\n`)
const warn = (msg) => warnings.push(msg)
const fail = (msg) => errors.push(msg)
const read = (locale) => readFileSync(join(ROOT, `src/shared/i18n/${locale}.ts`), 'utf8')

function lastTag() {
  try {
    return execFileSync('git', ['describe', '--tags', '--abbrev=0'], {
      cwd: ROOT,
      encoding: 'utf8',
    }).trim()
  } catch {
    return null
  }
}

function gitCatalog(locale, ref) {
  try {
    const src = execFileSync('git', ['show', `${ref}:src/shared/i18n/${locale}.ts`], {
      cwd: ROOT,
      encoding: 'utf8',
    })
    return parse(src).entries
  } catch {
    return null
  }
}

function parse(source) {
  const entries = new Map()
  const keys = []
  const re = /^\s*'([^']+)':\s*'((?:[^'\\]|\\.)*)',?/gm
  for (const match of source.matchAll(re)) {
    keys.push(match[1])
    entries.set(match[1], match[2])
  }
  const duplicates = [...new Set(keys.filter((key, index) => keys.indexOf(key) !== index))]
  return { entries, duplicates }
}

const current = Object.fromEntries(
  LOCALES.map((locale) => {
    const { entries, duplicates } = parse(read(locale))
    if (duplicates.length) fail(`${locale}.ts 存在重复词条：${duplicates.join(', ')}`)
    return [locale, entries]
  }),
)
const base = current[BASE]

if (!since) {
  warn('未找到 git tag，跳过语义漂移检查（可用 --since <ref> 指定基线）')
}

for (const locale of LOCALES) {
  if (locale === BASE) continue
  const catalog = current[locale]
  const missing = [...base.keys()].filter((key) => !catalog.has(key))
  const extra = [...catalog.keys()].filter((key) => !base.has(key))
  if (missing.length) fail(`${locale}.ts 缺少词条：${missing.slice(0, 8).join(', ')}`)
  if (extra.length) fail(`${locale}.ts 有多余词条：${extra.slice(0, 8).join(', ')}`)

  const bad = []
  for (const [key, value] of base) {
    if (!catalog.has(key)) continue
    const expected = (value.match(/\{(\w+)\}/g) ?? []).sort().join(',')
    const actual = (catalog.get(key).match(/\{(\w+)\}/g) ?? []).sort().join(',')
    if (expected !== actual) bad.push(key)
  }
  if (bad.length) fail(`${locale}.ts 占位符不一致：${bad.slice(0, 8).join(', ')}`)

  const empty = [...catalog].filter(([, value]) => value.trim() === '').map(([key]) => key)
  if (empty.length) fail(`${locale}.ts 存在空值：${empty.join(', ')}`)
}
if (!errors.length) ok(`${LOCALES.length} 个 locale 的 key / 占位符 / 空值检查通过`)

for (const locale of CJK_SCRIPTS) {
  const hits = [...current[locale]].filter(
    ([key, value]) => CJK.test(value) && !CJK_ALLOWED_KEYS.test(key) && !CJK_ALLOWED_VALUES.test(value),
  )
  if (hits.length) {
    warn(`${locale}.ts 含未翻译的中日韩文字：${hits.slice(0, 6).map(([key]) => key).join(', ')}`)
  }
}

const en = current.en
for (const locale of LOCALES) {
  if (locale === 'en') continue
  const hits = [...current[locale]].filter(
    ([key, value]) => en.get(key) === value && value.split(/\s+/).length >= 4 && !value.includes('{'),
  )
  if (hits.length) {
    warn(`${locale}.ts 有 ${hits.length} 条整句与英文相同：${hits.slice(0, 5).map(([key]) => key).join(', ')}`)
  }
}

for (const locale of PLURAL_LOCALES) {
  const hits = [...current[locale]]
    .filter(([, value]) => value.trimStart().startsWith('{count}'))
    .map(([key]) => key)
  if (hits.length) {
    warn(`${locale}.ts 的量词文案以 {count} 开头，单数时可能语法错误：${hits.join(', ')}`)
  }
}

if (since) {
  const oldZh = gitCatalog(BASE, since)
  const oldEn = gitCatalog('en', since)
  if (!oldZh || !oldEn) {
    warn(`无法读取 ${since} 的基线词条，跳过语义漂移检查`)
  } else {
    const changed = new Set()
    for (const [key, value] of current[BASE]) {
      if (oldZh.get(key) !== value) changed.add(key)
    }
    for (const [key, value] of current.en) {
      if (oldEn.get(key) !== value) changed.add(key)
    }

    const oldCatalogs = Object.fromEntries(LOCALES.map((locale) => [locale, gitCatalog(locale, since)]))
    const stale = new Map()
    for (const key of changed) {
      for (const locale of LOCALES) {
        if (locale === BASE || locale === 'en') continue
        const oldCatalog = oldCatalogs[locale]
        if (!oldCatalog || !oldCatalog.has(key)) continue
        if (current[locale].get(key) === oldCatalog.get(key)) {
          if (!stale.has(key)) stale.set(key, [])
          stale.get(key).push(locale)
        }
      }
    }

    if (changed.size === 0) {
      ok(`自 ${since} 以来 zh-CN / en 无文案变更`)
    } else if (stale.size === 0) {
      ok(`自 ${since} 以来改动的 ${changed.size} 条文案，其余 locale 均已更新`)
    } else {
      for (const [key, locales] of stale) {
        warn(`文案已改但未翻译：${key} -> ${locales.join(', ')}`)
      }
      process.stdout.write(`⚠ ${stale.size} 条文案需要人工确认翻译\n`)
    }
  }
}

if (warnings.length) {
  process.stdout.write(`\n⚠ ${warnings.length} 项待确认：\n`)
  for (const warning of warnings) process.stdout.write(`  - ${warning}\n`)
}

if (errors.length) {
  process.stdout.write(`\n✗ ${errors.length} 项错误：\n`)
  for (const error of errors) process.stdout.write(`  - ${error}\n`)
  process.exit(1)
}

if (strict && warnings.length) process.exit(1)
process.stdout.write('\n完成\n')
