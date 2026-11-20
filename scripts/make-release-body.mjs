#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import { existsSync, writeFileSync } from 'node:fs'
import { basename } from 'node:path'

function fail(message) {
  process.stderr.write(`\n❌ ${message}\n`)
  process.exit(1)
}

const opts = { repo: '', tag: '', out: 'release-body.md', assets: {} }
const argv = process.argv.slice(2)

for (let i = 0; i < argv.length; i += 2) {
  const flag = argv[i]
  const value = argv[i + 1]
  if (value === undefined) fail(`参数 ${flag} 缺少值`)

  switch (flag) {
    case '--repo':
      opts.repo = value
      break
    case '--tag':
      opts.tag = value
      break
    case '--out':
      opts.out = value
      break
    case '--asset': {
      const at = value.indexOf('=')
      if (at <= 0) fail(`--asset 需要 key=文件 形式：${value}`)
      opts.assets[value.slice(0, at)] = value.slice(at + 1)
      break
    }
    default:
      fail(`未知参数 ${flag}`)
  }
}

if (!opts.repo) fail('缺少 --repo')
if (!opts.tag) fail('缺少 --tag')

const ROWS = [
  { label: 'macOS (Apple Silicon)', keys: ['macos-pkg'] },
  { label: 'Windows (x64)', keys: ['windows-exe'] },
  { label: 'Linux (x64)', keys: ['linux-deb'] },
]

const BASE = `https://github.com/${opts.repo}/releases/download/${opts.tag}`

function links(keys) {
  return keys
    .filter((key) => opts.assets[key])
    .map((key) => {
      const path = opts.assets[key]
      if (!existsSync(path)) fail(`安装包不存在：${path}`)
      const name = basename(path)
      return `[${name}](${BASE}/${encodeURIComponent(name)})`
    })
    .join(' · ')
}

const rows = ROWS.map((row) => ({ label: row.label, links: links(row.keys) })).filter((row) => row.links)
if (rows.length === 0) fail('至少需要一个 --asset')

let prevTag = ''
try {
  prevTag = execFileSync('git', ['describe', '--tags', '--abbrev=0', '--exclude', opts.tag, 'HEAD'], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  }).trim()
} catch {
  prevTag = ''
}

const changes = []
if (prevTag) {
  const log = execFileSync('git', ['log', '--no-merges', '--reverse', '--format=- %s (%h)', `${prevTag}..${opts.tag}`], {
    encoding: 'utf8',
  })
  for (const line of log.split('\n')) {
    if (/^- (feat|fix|perf)(\(|:)/.test(line)) changes.push(line)
  }
}

const lines = ['## Download', '', '| Platform | Installer |', '| --- | --- |']
for (const row of rows) lines.push(`| ${row.label} | ${row.links} |`)

if (prevTag) {
  lines.push('', "## What's Changed", '')
  if (changes.length) lines.push(...changes, '')
  lines.push(`**Full Changelog**: https://github.com/${opts.repo}/compare/${prevTag}...${opts.tag}`)
}

writeFileSync(opts.out, `${lines.join('\n')}\n`)
process.stdout.write(`已生成 ${opts.out}：${rows.map((row) => row.label).join('、')}\n`)
