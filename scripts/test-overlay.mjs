import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'
import { test } from 'node:test'
import vm from 'node:vm'
import ts from 'typescript'
import React from 'react'
import { renderToStaticMarkup } from 'react-dom/server'

// 无浏览器、音频或模型依赖；用项目现有 TypeScript 编译器渲染真实组件。
const root = path.resolve(import.meta.dirname, '..')
const require = createRequire(import.meta.url)
const cache = new Map()
let locale = 'en'
const loads = []
const nativeCalls = []
let nativeFailure = null
let rootElement = null
let shallowHooks = null
let bloomState = null
const ipcCalls = []
let ipcAction = async () => {}
const mockIpc = {
  formatCmdError: String,
  EVENT: { taskState: 'task', overlayState: 'overlay', overlayShortcuts: 'shortcuts' },
  overlayConfigGet: async () => ({}), sessionStatus: async () => ({ status: 'capturing' }),
  shortcutsGet: async () => [],
  overlayConfigPatch: (patch) => { ipcCalls.push(['patch', patch]); return ipcAction() },
  overlayHide: () => { ipcCalls.push(['hide']); return ipcAction() },
  overlaySetMode: (mode) => { ipcCalls.push(['mode', mode]); return ipcAction() },
}
const nativeWindow = {
  startDragging: () => { nativeCalls.push(['drag']); return nativeFailure ? Promise.reject(nativeFailure) : Promise.resolve() },
  startResizeDragging: (direction) => { nativeCalls.push(['resize', direction]); return nativeFailure ? Promise.reject(nativeFailure) : Promise.resolve() },
}

function load(name, from = path.join(root, 'src')) {
  if (name === '@tauri-apps/api/window') return { getCurrentWindow: () => nativeWindow }
  if (name === '@/shared/ipc') return mockIpc
  if (name === 'react' && shallowHooks) return { ...React, ...shallowHooks }
  if (name === 'react-dom/client') return { createRoot: () => ({ render: (element) => { rootElement = element } }) }
  if (name === '@/shared/analytics') return { syncTrackingFromPrefs: async () => {} }
  if (name === '@/shared/locale') return { initLocale: async () => {} }
  if (name === '@/shared/useBroadcastSnapshot') return { useBroadcastSnapshot: () => {} }
  if (name === '@/shared/useTranscriptStream') return { useTranscriptStream: () => ({ segments: [baseSegment] }) }
  if (name === './FloralBorder' && shallowHooks) return { useEggBloom: () => ({ bloom: bloomState, finish() {} }), FloralBorder: () => null }
  if (name.endsWith('.css')) return {}

  if (name === '@/shared/i18n') return { useI18n: () => ({ t: (key, vars = {}) => {
    const messages = locale === 'zh-CN' ? load('@/shared/i18n/zh-CN').zhCN : load('@/shared/i18n/en').en
    return (messages[key] ?? key).replace(/\{(\w+)\}/g, (match, key) => vars[key] ?? match)
  } }) }
  if (!name.startsWith('@/') && !name.startsWith('.')) return require(name)
  const base = name.startsWith('@/') ? path.join(root, 'src', name.slice(2)) : path.resolve(from, name)
  const file = [base, `${base}.tsx`, `${base}.ts`, path.join(base, 'index.ts')].find((p) => fs.existsSync(p) && fs.statSync(p).isFile())
  if (!file) throw new Error(`测试模块不存在：${base}`)
  if (cache.has(file)) return cache.get(file).exports
  const module = { exports: {} }
  cache.set(file, module)
  const code = ts.transpileModule(fs.readFileSync(file, 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 } }).outputText
  vm.runInThisContext(`(function(require, module, exports) {${code}\n})`, { filename: file })((name) => load(name, path.dirname(file)), module, module.exports)
  loads.push(file)
  return module.exports
}
const { StreamView } = load('@/overlay/StreamView')
const { OverlayToolbar, ResizeHandle } = load('@/overlay/OverlayToolbar')
const baseSegment = { id: 'test:caption', text: 'Original sample', translation: '译文样本', translationState: 'done', state: 'final', revision: 1, startMs: 0, endMs: 10 }
const render = (displayMode, segment = baseSegment) => renderToStaticMarkup(React.createElement(StreamView, { segments: [segment], displayMode, fontSize: 22, fontColor: '#ffffff', opacity: 60, bgColor: '#101018' }))
const shortcuts = [{ id: 'overlay.toggleMode', acceleratorPretty: '⌘⇧K', registered: true }, { id: 'overlay.toggleVisible', acceleratorPretty: '⌘⇧J', registered: true }]
const toolbar = (mode = 'edit', keys = shortcuts, busy = false) => renderToStaticMarkup(React.createElement(OverlayToolbar, { config: { mode, displayMode: 'bilingual' }, shortcuts: keys, busy, onHide() {}, onLock() {}, onDisplayMode() {}, onError() {} }))

test('双语同时显示原文与译文', () => {
  const html = render('bilingual')
  assert.match(html, /Original sample/)
  assert.match(html, /译文样本/)
  assert.match(html, /font-size:0.8em/)
})
test('原文模式使用完整字号且不显示译文或翻译错误', () => {
  const html = render('source')
  assert.match(html, /Original sample/)
  assert.match(html, /font-size:1em;opacity:1/)
  assert.doesNotMatch(html, /译文样本/)
  assert.doesNotMatch(render('source', { ...baseSegment, translationState: 'failed' }), /Translation failed/)
})
test('译文模式不泄露原文，含失败、等待与空译文状态', () => {
  assert.doesNotMatch(render('translation'), /Original sample/)
  assert.match(render('translation'), /译文样本/)
  for (const [state, expected] of [['pending', 'Translating…'], ['idle', 'Waiting for translation…'], ['failed', 'Translation failed'], ['done', 'No translation for this segment']]) {
    const html = render('translation', { ...baseSegment, translation: '', translationState: state })
    assert.match(html, new RegExp(expected))
    assert.doesNotMatch(html, /Original sample/)
  }
})
test('编辑工具栏包含拖动、三档、锁定与隐藏，并使用实际改绑快捷键', () => {
  const html = toolbar()
  assert.equal((html.match(/<button/g) ?? []).length, 6)
  for (const label of ['Move overlay', 'Source', 'Both', 'Translation', '⌘⇧K', '⌘⇧J', 'height:40px']) assert.ok(html.includes(label), label)
  assert.match(html, /aria-pressed="true"/)
  assert.doesNotMatch(html, /⌘⇧L|⌘⇧H/)
})
test('锁定只有状态与恢复提示，不呈现无法点击的按钮', () => {
  const html = toolbar('locked')
  assert.match(html, /Locked · ⌘⇧K to unlock/)
  assert.match(html, /main window → Overlay/)
  assert.doesNotMatch(html, /<button/)
  assert.match(toolbar('locked', []), /Unlock in the main window → Overlay/)
})
test('快捷键注册失败时禁用浮窗锁定，忙时禁用可变控件', () => {
  assert.match(toolbar('edit', []), /disabled=""/)
  assert.match(toolbar('edit', []), /working unlock shortcut/)
  assert.ok((toolbar('edit', shortcuts, true).match(/disabled=""/g) ?? []).length >= 3)
})
test('中文控件走本地化且没有硬编码快捷键', () => {
  locale = 'zh-CN'
  try {
    const html = toolbar()
    for (const text of ['拖动', '原文', '双语', '译文', '不会停止转写', '⌘⇧K']) assert.ok(html.includes(text), text)
  } finally { locale = 'en' }
})
test('渲染组件没有导入音频、ASR或翻译服务', () => {
  assert.equal(loads.some((p) => /session|audio|asr/i.test(path.relative(root, p))), false)
})

function nodes(element, predicate) {
  if (!element || typeof element !== 'object') return []
  const children = React.Children.toArray(element.props?.children)
  return [...(predicate(element) ? [element] : []), ...children.flatMap(child => nodes(child, predicate))]
}
const tick = () => new Promise(resolve => setImmediate(resolve))

test('按钮只调用显示、锁定和隐藏回调', () => {
  const calls = []
  const element = OverlayToolbar({ config: { mode: 'edit', displayMode: 'bilingual' }, shortcuts, busy: false,
    onHide: () => calls.push('hide'), onLock: () => calls.push('lock'),
    onDisplayMode: (mode) => calls.push(mode), onError() {} })
  for (const button of nodes(element, node => node.type === 'button')) button.props.onClick?.()
  assert.deepEqual(calls, ['source', 'bilingual', 'translation', 'lock', 'hide'])
})

test('拖动只响应左键，原生失败会交给错误提示', async () => {
  nativeCalls.length = 0
  const errors = []
  const element = OverlayToolbar({ config: { mode: 'edit', displayMode: 'bilingual' }, shortcuts, busy: false,
    onHide() {}, onLock() {}, onDisplayMode() {}, onError: error => errors.push(error) })
  const drag = nodes(element, node => node.type === 'button')[0].props.onMouseDown
  drag({ button: 2, preventDefault() { throw new Error('右键不应取消默认事件') } })
  assert.equal(nativeCalls.length, 0)
  let prevented = false
  nativeFailure = new Error('模拟拖动失败')
  try {
    drag({ button: 0, preventDefault() { prevented = true } })
    await tick()
    assert.ok(prevented)
    assert.deepEqual(nativeCalls, [['drag']])
    assert.match(errors[0], /模拟拖动失败/)
  } finally { nativeFailure = null }
})

test('缩放方向随 RTL 改变，不声称实际窗口已移动', async () => {
  const original = globalThis.document
  nativeCalls.length = 0
  try {
    globalThis.document = { documentElement: { dir: 'rtl' } }
    ResizeHandle({ onError() {} }).props.onMouseDown({ button: 0, preventDefault() {} })
    globalThis.document.documentElement.dir = 'ltr'
    ResizeHandle({ onError() {} }).props.onMouseDown({ button: 0, preventDefault() {} })
    await tick()
    assert.deepEqual(nativeCalls, [['resize', 'SouthWest'], ['resize', 'SouthEast']])
  } finally { globalThis.document = original }
})

let App
let states = [], refs = [], stateIndex = 0, refIndex = 0
function renderApp() {
  stateIndex = 0; refIndex = 0
  return App()
}
function resetApp() {
  states = []; refs = []; bloomState = null; ipcCalls.length = 0; ipcAction = async () => {}
  return renderApp()
}

test('浮窗主组件保留整窗流光边框，bloom 时隐藏操作控件并保留花边', () => {
  shallowHooks = {
    useState(initial) {
      const index = stateIndex++
      if (!(index in states)) states[index] = initial
      return [states[index], value => { states[index] = typeof value === 'function' ? value(states[index]) : value }]
    },
    useRef(initial) {
      const index = refIndex++
      return refs[index] ??= { current: initial }
    },
  }
  const original = globalThis.document
  globalThis.document = { getElementById: () => ({}) }
  try { load('@/overlay/main') } finally { globalThis.document = original }
  App = rootElement.props.children.type
  resetApp()
  const configIndex = states.findIndex(state => state?.displayMode === 'bilingual')
  states[configIndex] = { ...states[configIndex], glow: true }
  const normal = renderApp()
  const panel = React.Children.toArray(normal.props.children)[0]
  assert.deepEqual(panel.props.style, { inset: 0 }, '普通状态没有动画预留边距')
  assert.equal(nodes(normal, node => node.type === OverlayToolbar).length, 1)
  assert.ok(React.Children.toArray(panel.props.children).some(node => typeof node.type === 'function' && node.type.name === 'FlowingBorder'), '流光边框必须是完整面板的直接子节点')
  bloomState = { id: 1, margin: 56, width: 640, height: 240, seed: 1 }
  const blooming = renderApp()
  assert.deepEqual(React.Children.toArray(blooming.props.children)[0].props.style, { left: 56, top: 56, width: 640, height: 240 })
  assert.equal(nodes(blooming, node => node.type === OverlayToolbar || node.type === ResizeHandle).length, 0)
  assert.equal(nodes(blooming, node => node.type?.name === 'FlowingBorder').length, 0)
  assert.equal(nodes(blooming, node => node.props?.bloom === bloomState).length, 1)
  bloomState = null
  assert.deepEqual(React.Children.toArray(renderApp().props.children)[0].props.style, { inset: 0 }, '动画结束恢复普通面板 bounds')
})

test('浮窗主组件在等待请求时忽略重复操作，失败后可恢复', async () => {
  let element = resetApp()
  let toolbar = nodes(element, node => node.type === OverlayToolbar)[0]
  let release
  ipcAction = () => new Promise(resolve => { release = resolve })
  toolbar.props.onDisplayMode('source')
  toolbar.props.onHide()
  assert.deepEqual(ipcCalls, [['patch', { displayMode: 'source' }]])
  element = renderApp()
  assert.equal(nodes(element, node => node.type === OverlayToolbar)[0].props.busy, true)
  release(); await tick()
  ipcAction = async () => { throw new Error('模拟保存失败') }
  toolbar.props.onDisplayMode('translation'); await tick()
  element = renderApp()
  assert.match(nodes(element, node => node.props?.role === 'alert')[0].props.children, /模拟保存失败/)
  ipcAction = async () => {}
  toolbar = nodes(element, node => node.type === OverlayToolbar)[0]
  toolbar.props.onHide(); await tick()
  element = renderApp()
  assert.equal(nodes(element, node => node.props?.role === 'alert').length, 0)
  assert.equal(nodes(element, node => node.type === OverlayToolbar)[0].props.busy, false)
  assert.deepEqual(ipcCalls.at(-1), ['hide'])
})
