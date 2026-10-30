#!/usr/bin/env node

import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const CATALOG = join(ROOT, 'src-tauri', 'assets', 'model-catalog.json')
const online = process.argv.includes('--online')

const errors = []
const check = (cond, msg) => {
  if (!cond) errors.push(msg)
}
const safeRel = (dir) =>
  typeof dir === 'string' &&
  dir.length > 0 &&
  !dir.includes('..') &&
  !dir.includes('\\') &&
  !dir.includes(':') &&
  dir.split('/').every((part) => /^[A-Za-z0-9._-]+$/.test(part))

const catalog = JSON.parse(await readFile(CATALOG, 'utf8'))
check(catalog.version === 1, `不支持的 catalog 版本：${catalog.version}`)

const ids = new Set()
for (const m of catalog.models) {
  check(!ids.has(m.id), `重复的模型 id：${m.id}`)
  ids.add(m.id)
  check(safeRel(m.install.dir), `${m.id} 的安装目录不安全：${m.install.dir}`)
  check(typeof m.title === 'string' && m.title.trim().length > 0, `${m.id} 缺少 title`)
  check(typeof m.description === 'string' && m.description.trim().length > 0, `${m.id} 缺少 description`)
  if (m.pruneProfile) {
    check(
      ['sherpa-asr-mixed-int8', 'sherpa-asr-all-int8'].includes(m.pruneProfile),
      `${m.id} 的剪裁方案未知：${m.pruneProfile}`,
    )
  }
  if (m.multilingual) {
    check(Array.isArray(m.languages) && m.languages.length > 0, `${m.id} 多语言模型必须声明 languages`)
  }
  if (m.languages) {
    check(
      Array.isArray(m.languages) && m.languages.every((code) => /^[a-z]+$/.test(code)),
      `${m.id} 的 languages 必须是语言代码数组`,
    )
  }
  if (m.archive) {
    check(m.archive.url.startsWith('https://'), `${m.id} 的下载地址不是 HTTPS`)
    if (m.downloadable) {
      check(/^[0-9a-f]{64}$/i.test(m.archive.sha256), `${m.id} 的 SHA-256 未填写`)
      check(m.archive.sizeBytes > 0, `${m.id} 的 sizeBytes 未填写`)
      check(m.archive.maxExtractedBytes > 0, `${m.id} 的 maxExtractedBytes 未填写`)
    }
  } else {
    check(!m.downloadable, `${m.id} 标记为可下载但没有 archive`)
  }
}

for (const [kind, tier] of [
  ['asr', 'builtin'],
  ['asr', 'high-accuracy'],
  ['asr', 'x-asr'],
  ['asr', 'nemotron'],
  ['asr', 'qwen3'],
  ['mt', 'base'],
  ['mt', 'multilingual'],
  ['mt', 'lmt-fast'],
  ['mt', 'lmt-high'],
]) {
  const count = catalog.models.filter((m) => m.kind === kind && m.tier === tier).length
  check(count === 1, `${kind}/${tier} 档必须恰好有一个模型（当前 ${count}）`)
}

if (online) {
  for (const m of catalog.models.filter((entry) => entry.downloadable)) {
    process.stdout.write(`⬇️  校验 ${m.id} …\n`)
    const res = await fetch(m.archive.url)
    if (!res.ok) {
      errors.push(`${m.id} 下载失败：HTTP ${res.status}`)
      continue
    }
    const hash = createHash('sha256')
    let size = 0
    for await (const chunk of res.body) {
      hash.update(chunk)
      size += chunk.length
    }
    const sha = hash.digest('hex')
    check(size === m.archive.sizeBytes, `${m.id} 体积不符：catalog ${m.archive.sizeBytes}，实际 ${size}`)
    check(
      sha.toLowerCase() === m.archive.sha256.toLowerCase(),
      `${m.id} SHA-256 不符：catalog ${m.archive.sha256}，实际 ${sha}`,
    )
    process.stdout.write(`   ${sha} (${size} bytes)\n`)
  }
}

if (errors.length > 0) {
  for (const e of errors) console.error(`❌ ${e}`)
  process.exit(1)
}
process.stdout.write(`✅ catalog 校验通过${online ? '（含在线校验）' : ''}\n`)
