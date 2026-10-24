#!/usr/bin/env node
import { appendFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const project = JSON.parse(readFileSync(resolve(root, 'project.json'), 'utf8'))
const publicKey = (process.env.TAURI_UPDATER_PUBLIC_KEY ?? '').trim()
const signingKeyPresent = Boolean(process.env.TAURI_SIGNING_PRIVATE_KEY?.trim())
const required = process.argv.includes('--require-updater') || /^refs\/tags\/v/.test(process.env.GITHUB_REF ?? '')
const outputFlag = process.argv.indexOf('--output')

function fail(message) {
  process.stderr.write(`发布配置错误：${message}\n`)
  process.exit(1)
}

if (publicKey) {
  const text = Buffer.from(publicKey, 'base64').toString('utf8').trim().split(/\r?\n/)
  const bytes = Buffer.from(text[1] ?? '', 'base64')
  if (!text[0]?.startsWith('untrusted comment:') || bytes.length !== 42 || !['Ed', 'ED'].includes(bytes.subarray(0, 2).toString())) {
    fail('TAURI_UPDATER_PUBLIC_KEY 必须是本项目 Tauri signer 生成的完整公钥文件内容（base64 格式）')
  }
}

if (required && (!publicKey || !signingKeyPresent)) {
  fail('tag 发布需要变量 TAURI_UPDATER_PUBLIC_KEY 和 Secret TAURI_SIGNING_PRIVATE_KEY；请先在仓库 Actions 设置中配置')
}

const enabled = Boolean(publicKey && signingKeyPresent)
if (outputFlag >= 0 && !process.argv[outputFlag + 1]) fail('--output 缺少输出路径')
const output = resolve(root, outputFlag >= 0 ? process.argv[outputFlag + 1] : '.cache/tauri-release.conf.json')
mkdirSync(dirname(output), { recursive: true })
writeFileSync(output, `${JSON.stringify({
  bundle: { createUpdaterArtifacts: enabled },
  plugins: { updater: { pubkey: publicKey, endpoints: [project.updaterEndpoint] } },
}, null, 2)}\n`)

if (process.env.GITHUB_OUTPUT) {
  appendFileSync(process.env.GITHUB_OUTPUT, `updater_enabled=${enabled}\n`)
}
process.stdout.write(enabled ? '本次构建启用本项目签名更新包。\n' : '本次构建不生成更新包；未配置公钥的应用将禁用自动更新。\n')
