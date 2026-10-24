import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, existsSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'

const script = fileURLToPath(new URL('./configure-release.mjs', import.meta.url))
const links = JSON.parse(readFileSync(new URL('../project.json', import.meta.url), 'utf8'))
const publicBytes = Buffer.alloc(42)
publicBytes.write('Ed')
const publicKey = Buffer.from(`untrusted comment: synthetic public key fixture\n${publicBytes.toString('base64')}\n`).toString('base64')
const privateFixture = 'SYNTHETIC_SIGNING_FIXTURE_DO_NOT_PERSIST'

function run(t, env = {}, args = []) {
  const dir = mkdtempSync(join(tmpdir(), 'audioscript-release-config-'))
  t.after(() => rmSync(dir, { recursive: true, force: true }))
  const output = join(dir, 'config.json')
  const result = spawnSync(process.execPath, [script, '--output', output, ...args], {
    encoding: 'utf8',
    env: {
      ...process.env,
      GITHUB_REF: '',
      GITHUB_OUTPUT: join(dir, 'github-output'),
      TAURI_UPDATER_PUBLIC_KEY: '',
      TAURI_SIGNING_PRIVATE_KEY: '',
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD: '',
      ...env,
    },
  })
  return { ...result, output, dir }
}

test('普通构建未配置密钥时关闭更新工件，使用 fork endpoint', (t) => {
  const result = run(t)
  assert.equal(result.status, 0, result.stderr)
  const config = JSON.parse(readFileSync(result.output, 'utf8'))
  assert.equal(config.bundle.createUpdaterArtifacts, false)
  assert.equal(config.plugins.updater.pubkey, '')
  assert.deepEqual(config.plugins.updater.endpoints, [links.updaterEndpoint])
  assert.equal(readFileSync(join(result.dir, 'github-output'), 'utf8'), 'updater_enabled=false\n')
})

test('tag 缺公钥或私钥时拒绝发布且不写配置', (t) => {
  for (const env of [{}, { TAURI_UPDATER_PUBLIC_KEY: publicKey }, { TAURI_SIGNING_PRIVATE_KEY: privateFixture }]) {
    const result = run(t, { GITHUB_REF: 'refs/tags/v0.2.10', ...env })
    assert.notEqual(result.status, 0)
    assert.match(result.stderr, /tag 发布需要/)
    assert.equal(existsSync(result.output), false)
    assert.equal(`${result.stdout}${result.stderr}`.includes(privateFixture), false)
  }
})

test('自有签名配置只将公钥写入更新配置，不持久化或输出私钥', (t) => {
  const result = run(t, {
    TAURI_UPDATER_PUBLIC_KEY: publicKey,
    TAURI_SIGNING_PRIVATE_KEY: privateFixture,
  }, ['--require-updater'])
  assert.equal(result.status, 0, result.stderr)
  const raw = readFileSync(result.output, 'utf8')
  const config = JSON.parse(raw)
  assert.equal(config.bundle.createUpdaterArtifacts, true)
  assert.equal(config.plugins.updater.pubkey, publicKey)
  assert.deepEqual(config.plugins.updater.endpoints, [links.updaterEndpoint])
  assert.equal(`${raw}${result.stdout}${result.stderr}`.includes(privateFixture), false)
  assert.equal(readFileSync(join(result.dir, 'github-output'), 'utf8'), 'updater_enabled=true\n')
})

test('无效公钥拒绝构建，不泄露传入值', (t) => {
  const invalid = 'INVALID_PUBLIC_KEY_FIXTURE'
  const result = run(t, { TAURI_UPDATER_PUBLIC_KEY: invalid, TAURI_SIGNING_PRIVATE_KEY: privateFixture })
  assert.notEqual(result.status, 0)
  assert.match(result.stderr, /TAURI_UPDATER_PUBLIC_KEY 必须/)
  assert.equal(existsSync(result.output), false)
  assert.equal(`${result.stdout}${result.stderr}`.includes(invalid), false)
  assert.equal(`${result.stdout}${result.stderr}`.includes(privateFixture), false)
})
