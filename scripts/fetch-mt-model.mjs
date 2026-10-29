#!/usr/bin/env node

import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { createReadStream } from 'node:fs'
import { mkdir, readdir, rm, stat } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const VENV = join(ROOT, '.cache', 'ct2venv')

const IS_WIN = process.platform === 'win32'
const VENV_BIN = join(VENV, IS_WIN ? 'Scripts' : 'bin')
const PYTHON = IS_WIN ? 'python' : 'python3'
const PY = join(VENV_BIN, IS_WIN ? 'python.exe' : 'python')
const TOK_CACHE = join(ROOT, '.cache', 'mt')

const force = process.argv.includes('--force')
const multilingual = process.argv.includes('--multilingual')

const MT_DIR = join(ROOT, 'src-tauri', 'assets', 'models', 'mt')
const NLLB_HF = 'facebook/nllb-200-distilled-600M'
const NLLB_DIR_NAME = 'nllb-200-distilled-600M-ct2-int8'
const NLLB_ROOT = join(ROOT, '.cache', 'mt-nllb')
const NLLB_OUT = join(NLLB_ROOT, NLLB_DIR_NAME)
const NLLB_ARCHIVE = join(ROOT, '.cache', `${NLLB_DIR_NAME}.tar.gz`)
const NLLB_URL = `https://github.com/niyongsheng/AudioScript/releases/download/models-v1/${NLLB_DIR_NAME}.tar.gz`

const PAIRS = [
  {
    hf: 'Helsinki-NLP/opus-mt-en-zh',
    dir: 'opus-mt-en-zh-ct2',
    tokenizer: 'tok-en-zh',
    note: 'en→zh (Apache-2.0)',
  },
  {
    hf: 'Helsinki-NLP/opus-mt-zh-en',
    dir: 'opus-mt-zh-en-ct2',
    tokenizer: 'tok-zh-en',
    note: 'zh→en (CC-BY-4.0，需署名)',
  },
]

const SPM_TO_JSON_SCRIPT = `
import sys, os, sentencepiece
sys.path.insert(0, os.path.dirname(sentencepiece.__file__))
from tokenizers import SentencePieceUnigramTokenizer
tok = SentencePieceUnigramTokenizer.from_spm(sys.argv[1])
tok.save(sys.argv[2])
`

async function exists(p) {
  try {
    await stat(p)
    return true
  } catch {
    return false
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

async function hasFiles(dir, files) {
  for (const f of files) {
    if (!(await exists(join(dir, f)))) return false
  }
  return true
}

async function isReady(dir) {
  return hasFiles(dir, ['model.bin', 'source-tokenizer.json', 'target-tokenizer.json'])
}

function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { stdio: 'inherit', ...opts })
  if (r.error) throw new Error(`无法执行 ${cmd}：${r.error.message}`)
  if (r.status !== 0) throw new Error(`${cmd} 退出码 ${r.status}`)
}

async function ensureVenv() {
  if (!(await exists(PY))) {
    process.stdout.write('🐍 创建 Python venv（一次性的，约 1-2 分钟）…\n')
    await mkdir(join(ROOT, '.cache'), { recursive: true })
    run(PYTHON, ['-m', 'venv', VENV])
    run(PY, ['-m', 'pip', 'install', '--quiet', '--upgrade', 'pip'])
  }

  const probe = spawnSync(
    PY,
    ['-c', 'import torch, ctranslate2, transformers, sentencepiece, google.protobuf'],
    { stdio: 'ignore' },
  )
  if (probe.status !== 0) {
    const v = spawnSync(PY, ['-V'], { encoding: 'utf8' })
    const version = (v.stdout || v.stderr || '').trim()
    process.stdout.write(`📦 安装转换依赖（torch 较大，首次约 3-5 分钟）… [${version}]\n`)
    run(PY, [
      '-m',
      'pip',
      'install',
      'torch',
      'ctranslate2',
      'transformers',
      'sentencepiece',
      'protobuf',
    ])
  }
}

async function fetchVocab(pair) {
  const out = join(TOK_CACHE, pair.tokenizer)
  if (!force && (await exists(join(out, 'source.spm')))) return out

  await mkdir(out, { recursive: true })
  process.stdout.write(`🔤 下载词表：${pair.hf}\n`)
  run(PY, [
    '-c',
    `from transformers import AutoTokenizer; \
AutoTokenizer.from_pretrained(${JSON.stringify(pair.hf)}).save_pretrained(${JSON.stringify(out)})`,
  ])
  return out
}

async function convert(pair) {
  const out = join(MT_DIR, pair.dir)
  if (!force && (await isReady(out))) {
    const size = await dirSize(out)
    process.stdout.write(
      `✅ ${pair.note} 已就绪（${(size / 1024 / 1024).toFixed(0)} MB）\n`,
    )
    return
  }

  await ensureVenv()
  const tokDir = await fetchVocab(pair)

  await mkdir(MT_DIR, { recursive: true })
  await rm(out, { recursive: true, force: true })

  process.stdout.write(`⬇️  转换 ${pair.note}（下载 fp32 权重 + 转 int8）…\n`)
  run(join(VENV_BIN, IS_WIN ? 'ct2-transformers-converter.exe' : 'ct2-transformers-converter'), [
    '--model',
    pair.hf,
    '--output_dir',
    out,
    '--quantization',
    'int8',
  ])

  for (const [spm, outName] of [
    ['source.spm', 'source-tokenizer.json'],
    ['target.spm', 'target-tokenizer.json'],
  ]) {
    process.stdout.write(`🔤 生成词表：${outName}\n`)
    run(PY, ['-c', SPM_TO_JSON_SCRIPT, join(tokDir, spm), join(out, outName)])
  }

  if (!(await isReady(out))) {
    throw new Error(`转换后仍缺文件：${out}`)
  }
  const size = await dirSize(out)
  process.stdout.write(`✅ ${pair.note} 完成（${(size / 1024 / 1024).toFixed(0)} MB）\n`)
}

async function sha256File(path) {
  const hash = createHash('sha256')
  for await (const chunk of createReadStream(path)) hash.update(chunk)
  return hash.digest('hex')
}

async function convertMultilingual() {
  const required = ['model.bin', 'shared_vocabulary.json', 'config.json', 'tokenizer.json']
  const complete = (dir) => hasFiles(dir, required)

  if (force || !(await complete(NLLB_OUT))) {
    await ensureVenv()
    await mkdir(NLLB_ROOT, { recursive: true })
    await rm(NLLB_OUT, { recursive: true, force: true })
    process.stdout.write(`⬇️  转换 ${NLLB_HF}（下载 fp32 权重 + 转 int8，首次较久）…\n`)
    run(join(VENV_BIN, IS_WIN ? 'ct2-transformers-converter.exe' : 'ct2-transformers-converter'), [
      '--model',
      NLLB_HF,
      '--output_dir',
      NLLB_OUT,
      '--quantization',
      'int8',
      '--copy_files',
      'tokenizer.json',
      'special_tokens_map.json',
      '--force',
    ])
    if (!(await complete(NLLB_OUT))) {
      throw new Error(`转换后仍缺文件：${NLLB_OUT}`)
    }
  }

  const installed = await dirSize(NLLB_OUT)
  process.stdout.write(`📦 打包 ${NLLB_ARCHIVE}\n`)
  run('tar', ['-C', NLLB_ROOT, '-czf', NLLB_ARCHIVE, NLLB_DIR_NAME])
  const archiveSize = (await stat(NLLB_ARCHIVE)).size
  const sha = await sha256File(NLLB_ARCHIVE)

  process.stdout.write(`\n✅ NLLB 完成（安装后 ${(installed / 1024 / 1024).toFixed(0)} MB）\n`)
  process.stdout.write(
    [
      '',
      '上传到 GitHub Release：',
      `  gh release create models-v1 ${NLLB_ARCHIVE} --title "Models v1" --notes "可选模型资源"`,
      `  # release 已存在时：gh release upload models-v1 ${NLLB_ARCHIVE}`,
      '',
      'catalog 字段：',
      JSON.stringify(
        {
          archive: {
            url: NLLB_URL,
            format: 'tar-gz',
            sha256: sha,
            sizeBytes: archiveSize,
            maxExtractedBytes: Math.ceil(installed * 1.1),
          },
          sizes: { installedBytes: installed },
        },
        null,
        2,
      ),
      '',
    ].join('\n'),
  )
}

async function main() {
  if (multilingual) {
    process.stdout.write('多语言机器翻译模型（NLLB-200 + CTranslate2 int8）\n\n')
    await convertMultilingual()
    return
  }

  process.stdout.write('内置机器翻译模型（opus-mt + CTranslate2 int8）\n\n')

  for (const pair of PAIRS) {
    await convert(pair)
  }

  const total = await dirSize(MT_DIR)
  process.stdout.write(`\n✅ 翻译模型全部就绪，合计 ${(total / 1024 / 1024).toFixed(0)} MB\n`)
}

main().catch((e) => {
  console.error(`\n❌ ${e.message}`)
  process.exit(1)
})
