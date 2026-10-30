#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../../../..')
const LOCALES = ['en', 'zh-CN', 'zh-TW', 'ja', 'ko', 'fr', 'de', 'es']

const args = process.argv.slice(2)
const flags = new Set(args.filter((a) => a.startsWith('--')))
const version = (args.find((a) => !a.startsWith('--')) ?? '').replace(/^v/, '')
const post = flags.has('--post')
const skipWhatsNew = flags.has('--no-whats-new')

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  process.stderr.write(
    '用法：node .claude/skills/audioscript-release/scripts/check-release.mjs <X.Y.Z> [--no-whats-new] [--post]\n',
  )
  process.exit(2)
}

const problems = []
const warnings = []

const fail = (msg) => problems.push(msg)
const warn = (msg) => warnings.push(msg)
const ok = (msg) => process.stdout.write(`✓ ${msg}\n`)
const read = (rel) => readFileSync(join(ROOT, rel), 'utf8')

function checkWhatsNew() {
  const src = read('src/main/WhatsNew.tsx')
  const entry = new RegExp(`['"]${version.replace(/\./g, '\\.')}['"]\\s*:`)

  if (skipWhatsNew) {
    if (entry.test(src)) warn(`已声明跳过 What's New，但 WHATS_NEW 里仍有 '${version}' 条目`)
    else ok("What's New 已显式跳过")
    return
  }

  if (!entry.test(src)) {
    fail(`src/main/WhatsNew.tsx 的 WHATS_NEW 缺少 '${version}' 条目（确认无需弹窗时传 --no-whats-new）`)
    return
  }
  ok(`What's New 已包含 '${version}'`)

  const referenced = [...src.matchAll(/(?:titleKey|bodyKey):\s*'([^']+)'/g)].map((m) => m[1])
  const en = new Set([...read('src/shared/i18n/en.ts').matchAll(/'([^']+)'\s*:/g)].map((m) => m[1]))
  const missing = [...new Set(referenced)].filter((key) => !en.has(key))
  if (missing.length) fail(`WhatsNew.tsx 引用了 en.ts 不存在的词条：${missing.join(', ')}`)
}

function checkLocaleParity() {
  const keyRe = /'(whatsNew\.[A-Za-z0-9]+)'\s*:/g
  const sets = new Map()
  for (const locale of LOCALES) {
    const src = read(`src/shared/i18n/${locale}.ts`)
    sets.set(locale, new Set([...src.matchAll(keyRe)].map((m) => m[1])))
  }

  const base = sets.get('en')
  let mismatched = false
  for (const [locale, keys] of sets) {
    if (locale === 'en') continue
    const missing = [...base].filter((key) => !keys.has(key))
    const extra = [...keys].filter((key) => !base.has(key))
    if (missing.length || extra.length) {
      mismatched = true
      if (missing.length) fail(`${locale}.ts 缺少词条：${missing.join(', ')}`)
      if (extra.length) fail(`${locale}.ts 多出 en.ts 没有的词条：${extra.join(', ')}`)
    }
  }
  if (!mismatched) ok(`8 个 locale 的 whatsNew.* 词条一致（${base.size} 条）`)
}

function checkSite() {
  const site = read('docs/index.html')

  const software = site.match(/"softwareVersion":\s*"([^"]+)"/)?.[1]
  if (software !== version) {
    fail(`docs/index.html 的 softwareVersion 是 ${software ?? '缺失'}，应为 ${version}`)
  } else {
    ok(`官网 softwareVersion = ${version}`)
  }

  const staticVersion = site.match(/<span data-version>\s*v?([0-9.]+)\s*<\/span>/)?.[1]
  if (staticVersion !== version) {
    fail(`docs/index.html 的 data-version 是 ${staticVersion ?? '缺失'}，应为 ${version}`)
  } else {
    ok(`官网静态版本行 = v${version}`)
  }

  const iso = site.match(/data-date\s+data-iso="([^"]+)"/)?.[1]
  if (!iso) fail('docs/index.html 缺少 data-date data-iso')
  else if (!/^\d{4}-\d{2}-\d{2}$/.test(iso)) fail(`docs/index.html 的 data-date 不是 ISO 日期：${iso}`)
  else ok(`官网发布日期 = ${iso}`)
}

function checkCatalog() {
  const catalog = JSON.parse(read('src-tauri/assets/model-catalog.json'))
  const downloadable = catalog.models.filter((model) => model.downloadable === true)
  let broken = false

  for (const model of downloadable) {
    const archive = model.archive ?? {}
    if (!/^https:\/\//.test(archive.url ?? '')) {
      broken = true
      fail(`${model.id} 的下载地址不是 HTTPS`)
    }
    if (!/^[0-9a-f]{64}$/.test(archive.sha256 ?? '')) {
      broken = true
      fail(`${model.id} 缺少合法 sha256`)
    }
    if (!(archive.sizeBytes > 0)) {
      broken = true
      fail(`${model.id} 缺少 sizeBytes`)
    }
  }
  if (!broken) ok(`catalog 可下载模型 ${downloadable.length} 个，url / sha256 / sizeBytes 齐全`)
}

function checkPost() {
  const versions = new Map()

  const pkg = JSON.parse(read('package.json'))
  versions.set('package.json', pkg.version)

  const tauri = JSON.parse(read('src-tauri/tauri.conf.json'))
  versions.set('src-tauri/tauri.conf.json', tauri.version)

  versions.set(
    'src-tauri/Cargo.toml',
    read('src-tauri/Cargo.toml').match(/^version = "([^"]+)"/m)?.[1],
  )

  versions.set(
    'src-tauri/Cargo.lock',
    /\[\[package\]\]\nname = "audioscript"\nversion = "([^"]+)"/.exec(read('src-tauri/Cargo.lock'))?.[1],
  )

  let mismatch = false
  for (const [file, value] of versions) {
    if (value !== version) {
      mismatch = true
      fail(`${file} 的版本是 ${value ?? '缺失'}，应为 ${version}`)
    }
  }
  if (!mismatch) ok(`四处版本号均为 ${version}`)

  const tag = execFileSync('git', ['tag', '-l', `v${version}`], { cwd: ROOT, encoding: 'utf8' }).trim()
  if (!tag) fail(`本地缺少 tag v${version}`)
  else ok(`tag v${version} 已存在`)

  const dirty = execFileSync('git', ['status', '--porcelain'], { cwd: ROOT, encoding: 'utf8' }).trim()
  if (dirty) warn(`发布后工作区仍有改动（可能是 lint 自动修复）：\n${dirty}`)
  else ok('发布后工作区干净')
}

checkWhatsNew()
checkLocaleParity()
checkSite()
checkCatalog()
if (post) checkPost()

for (const message of warnings) process.stdout.write(`⚠ ${message}\n`)
for (const message of problems) process.stdout.write(`✗ ${message}\n`)

if (problems.length) {
  process.stdout.write(`\n${problems.length} 项未通过\n`)
  process.exit(1)
}

process.stdout.write('\n全部检查通过\n')
