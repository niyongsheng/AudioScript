#!/usr/bin/env node

import { createWriteStream } from 'node:fs'
import { mkdir, readdir, rename, rm, stat, unlink } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import { pipeline } from 'node:stream/promises'
import { Readable } from 'node:stream'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')

const argv = process.argv.slice(2)
const argValue = (name) => {
  const i = argv.indexOf(name)
  return i >= 0 ? argv[i + 1] : undefined
}
const high = argValue('--model') === 'high'
const printCatalog = argv.includes('--print-catalog')
const force = argv.includes('--force')

const MODEL = high
  ? 'sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20'
  : 'sherpa-onnx-streaming-zipformer-small-bilingual-zh-en-2023-02-16'
const URL = `https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/${MODEL}.tar.bz2`

const MODELS_DIR = join(ROOT, 'src-tauri', 'assets', 'models')
const CACHE_DIR = join(ROOT, '.cache')
const VAD_FILE = 'silero_vad.onnx'
const VAD_URL = `https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/${VAD_FILE}`
const VAD_SHA = '9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6'
const VAD_PATH = join(MODELS_DIR, VAD_FILE)
const TARGET_DIR = high ? join(CACHE_DIR, 'asr-high') : join(MODELS_DIR, MODEL)
const EXTRACTED_DIR = join(dirname(TARGET_DIR), MODEL)
const ARCHIVE = join(CACHE_DIR, `${MODEL}.tar.bz2`)

async function missingFiles() {
  let entries
  try {
    entries = await readdir(TARGET_DIR)
  } catch {
    return ['(模型目录不存在)']
  }

  const missing = []
  for (const prefix of ['encoder', 'decoder', 'joiner']) {
    if (!entries.some((n) => n.startsWith(prefix) && n.endsWith('.onnx'))) {
      missing.push(`${prefix}*.onnx`)
    }
  }
  if (!entries.includes('tokens.txt')) missing.push('tokens.txt')
  return missing
}

async function exists(p) {
  try {
    await stat(p)
    return true
  } catch {
    return false
  }
}

async function fetchTo(url, dest) {
  const res = await fetch(url, { redirect: 'follow' })
  if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText}`)
  await pipeline(Readable.fromWeb(res.body), createWriteStream(dest))
}

async function download() {
  await mkdir(CACHE_DIR, { recursive: true })
  process.stdout.write(`⬇️  下载 ${MODEL}\n    ${URL}\n`)

  try {
    await fetchTo(URL, ARCHIVE)
  } catch (e) {
    console.error(`❌ 下载失败：${e.message}`)
    process.exit(1)
  }
  process.stdout.write(`✅ 已缓存到 ${ARCHIVE}\n`)
}

async function ensureVad() {
  if (await exists(VAD_PATH)) {
    if ((await sha256File(VAD_PATH)) === VAD_SHA) return
  }

  await mkdir(MODELS_DIR, { recursive: true })
  await mkdir(CACHE_DIR, { recursive: true })
  const tmp = join(CACHE_DIR, VAD_FILE)
  process.stdout.write(`⬇️  下载 Silero VAD\n    ${VAD_URL}\n`)

  try {
    await fetchTo(VAD_URL, tmp)
    if ((await sha256File(tmp)) !== VAD_SHA) throw new Error('SHA-256 不匹配')
    await rm(VAD_PATH, { force: true })
    await rename(tmp, VAD_PATH)
    process.stdout.write(`✅ VAD 就绪：${VAD_PATH}\n`)
  } catch (e) {
    await rm(tmp, { force: true })
    process.stdout.write(`⚠️  VAD 下载失败（Qwen3-ASR 需要它）：${e.message}\n`)
  }
}

function extract() {
  const r = spawnSync('tar', ['-xjf', ARCHIVE, '-C', dirname(TARGET_DIR)], {
    stdio: 'inherit',
  })
  if (r.error) {
    console.error(`❌ 无法调用 tar：${r.error.message}`)
    console.error('   macOS / Linux 自带；Windows 需 10 1803 以上')
    process.exit(1)
  }
  if (r.status !== 0) {
    console.error(`❌ 解压失败（tar 退出码 ${r.status}）`)
    process.exit(1)
  }
}

async function prune() {
  const KEEP = [
    (n) => n.startsWith('encoder') && n.endsWith('.int8.onnx'),
    (n) => n.startsWith('decoder') && n.endsWith('.onnx') && !n.includes('.int8.'),
    (n) => n.startsWith('joiner') && n.endsWith('.int8.onnx'),
    (n) => n === 'tokens.txt',
    (n) => n === 'bpe.model',
  ]

  const entries = await readdir(TARGET_DIR, { withFileTypes: true })
  let removed = 0
  let freed = 0

  for (const e of entries) {
    if (e.isDirectory()) {
      freed += await dirSize(join(TARGET_DIR, e.name))
      await rm(join(TARGET_DIR, e.name), { recursive: true, force: true })
      removed++
      continue
    }
    if (!KEEP.some((pred) => pred(e.name))) {
      const p = join(TARGET_DIR, e.name)
      freed += (await stat(p)).size
      await unlink(p)
      removed++
    }
  }

  if (removed > 0) {
    process.stdout.write(
      `🧹 剪裁 ${removed} 项，省下 ${(freed / 1024 / 1024).toFixed(1)} MB\n`,
    )
  }
}

async function dirSize(dir) {
  let total = 0
  for (const e of await readdir(dir, { withFileTypes: true })) {
    const p = join(dir, e.name)
    total += e.isDirectory() ? await dirSize(p) : (await stat(p)).size
  }
  return total
}

async function sha256File(path) {
  const { createHash } = await import('node:crypto')
  const { createReadStream } = await import('node:fs')
  return new Promise((resolvePromise, reject) => {
    const hash = createHash('sha256')
    createReadStream(path)
      .on('data', (chunk) => hash.update(chunk))
      .on('error', reject)
      .on('end', () => resolvePromise(hash.digest('hex')))
  })
}

async function reportCatalog() {
  if (!high) return
  if (!(await exists(ARCHIVE))) await download()
  const sha = await sha256File(ARCHIVE)
  const archiveSize = (await stat(ARCHIVE)).size
  const installed = await dirSize(TARGET_DIR)
  if (printCatalog) {
    process.stdout.write(
      `\nCATALOG_FRAGMENT\n${JSON.stringify(
        { id: 'asr-high-accuracy', sha256: sha, sizeBytes: archiveSize, installedBytes: installed },
        null,
        2,
      )}\n`,
    )
  } else {
    process.stdout.write(`sha256=${sha}\nsizeBytes=${archiveSize}\ninstalledBytes=${installed}\n`)
  }
}

async function main() {
  await ensureVad()

  if (!force) {
    const missing = await missingFiles()
    if (missing.length === 0) {
      await prune()
      process.stdout.write(`✅ 模型已就绪：${TARGET_DIR}\n`)
      await reportCatalog()
      return
    }
  }

  await mkdir(dirname(TARGET_DIR), { recursive: true })
  await rm(TARGET_DIR, { recursive: true, force: true })

  if (force || !(await exists(ARCHIVE))) {
    await download()
  } else {
    process.stdout.write(`↩️  复用缓存 ${ARCHIVE}\n`)
  }

  extract()
  if (EXTRACTED_DIR !== TARGET_DIR) {
    await rm(TARGET_DIR, { recursive: true, force: true })
    await rename(EXTRACTED_DIR, TARGET_DIR)
  }
  await prune()

  const missing = await missingFiles()
  if (missing.length > 0) {
    console.error(`❌ 剪裁后缺文件：${missing.join(', ')}`)
    console.error(`   检查 ${TARGET_DIR} 的实际内容；`)
    console.error('   若该模型没有 int8 版本，需要放宽 scripts/fetch-model.mjs 的 KEEP 规则')
    process.exit(1)
  }

  const size = await dirSize(TARGET_DIR)
  process.stdout.write(
    `✅ 模型就绪：${TARGET_DIR}\n   实占 ${(size / 1024 / 1024).toFixed(1)} MB\n`,
  )
  await reportCatalog()
}

main().catch((e) => {
  console.error('❌ 未预期的失败：', e)
  process.exit(1)
})
