#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8')
const project = JSON.parse(read('project.json'))
const config = JSON.parse(read('src-tauri/tauri.conf.json'))
const catalog = JSON.parse(read('src-tauri/assets/model-catalog.json'))

assert.notEqual(project.repository, project.upstreamRepository, 'fork 仓库必须与原项目分开')
for (const url of Object.values(project)) assert.equal(new URL(url).protocol, 'https:', '项目链接必须使用 HTTPS')
assert.equal(project.updaterEndpoint, `${project.repository}/releases/latest/download/latest.json`)
assert.equal(project.modelsRelease, `${project.repository}/releases/download/models-v1`)
assert.deepEqual(config.plugins.updater.endpoints, [project.updaterEndpoint], '更新地址必须与 project.json 一致')
assert.equal(config.plugins.updater.pubkey, '', '源码不得内置上游或其他更新公钥，发布时注入自有公钥')
assert.equal(config.bundle.createUpdaterArtifacts, false, '普通构建默认不生成更新工件')
assert.equal(config.identifier, 'io.github.zyg0733.audioscript', 'fork 应用标识不能与上游共用')
assert.equal(catalog.models.find((model) => model.id === 'mt-big').archive.url.startsWith(`${project.modelsRelease}/`), true)

for (const path of ['README.md', 'docs/index.html']) {
  const source = read(path)
  assert.equal(source.includes(`${project.repository}/releases/latest`), true, `${path} 缺少 fork 下载入口`)
  assert.equal(source.includes(project.site), true, `${path} 的官网链接未更新`)
  assert.equal(source.includes(`${project.upstreamRepository}/releases`), false, `${path} 仍引导用户下载原项目`)
  assert.equal(source.includes(project.upstreamRepository), true, `${path} 必须保留原项目归属说明`)
}
process.stdout.write('项目身份、更新、模型来源与用户入口一致。\n')
