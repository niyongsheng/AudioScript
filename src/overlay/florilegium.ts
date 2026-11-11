const TAU = Math.PI * 2

type Rgb = readonly [number, number, number]

interface Palette {
  stem: Rgb
  stemLight: Rgb
  leaf: Rgb
  leafDeep: Rgb
  leafLight: Rgb
  petalDeep: Rgb
  petalMid: Rgb
  petalLight: Rgb
  petalEdge: Rgb
  pollen: Rgb
  pollenLight: Rgb
  wing: Rgb
  wingEdge: Rgb
}

const NIGHT_INK: Palette = {
  stem: [92, 142, 196],
  stemLight: [190, 216, 250],
  leaf: [108, 154, 208],
  leafDeep: [64, 114, 172],
  leafLight: [172, 204, 246],
  petalDeep: [38, 88, 170],
  petalMid: [72, 132, 222],
  petalLight: [162, 198, 250],
  petalEdge: [198, 222, 255],
  pollen: [224, 160, 94],
  pollenLight: [250, 208, 152],
  wing: [216, 152, 90],
  wingEdge: [246, 200, 144],
}

const PAPER_INK: Palette = {
  stem: [47, 95, 159],
  stemLight: [29, 63, 143],
  leaf: [73, 122, 180],
  leafDeep: [29, 63, 143],
  leafLight: [122, 162, 216],
  petalDeep: [29, 63, 143],
  petalMid: [36, 86, 196],
  petalLight: [126, 166, 234],
  petalEdge: [20, 48, 120],
  pollen: [180, 102, 47],
  pollenLight: [201, 162, 39],
  wing: [180, 102, 47],
  wingEdge: [143, 74, 28],
}

const GROW_MS = 3600
const FLY_AT = 2000
const FLY_MS = 2800
const LEAF_MS = 440
const FLOWER_MS = 950
const FADE_MS = 1300
const FADE_AT = 7200
const FADE_AT_STILL = 3800

interface Point {
  x: number
  y: number
}

interface FrameSample extends Point {
  tx: number
  ty: number
}

interface Frame {
  length: number
  at: (u: number) => FrameSample
}

interface VineSample extends Point {
  dx: number
  dy: number
}

interface LeafSpec {
  f: number
  side: number
  size: number
  angle: number
  tendril: boolean
  seed: number
}

interface FlowerSpec {
  f: number
  size: number
  seed: number
}

interface BudSpec {
  f: number
  side: number
  size: number
  seed: number
}

interface VineSpec {
  forward: boolean
  delay: number
  duration: number
  wavePhase: number
  waveAmp: number
  waveLength: number
  leaves: LeafSpec[]
  flowers: FlowerSpec[]
  buds: BudSpec[]
}

interface Garland {
  frame: Frame
  vines: VineSpec[]
  panelMin: number
  tipSize: number
  tipSeed: number
  tipAt: number
}

interface Visitor {
  start: Point
  c1: Point
  c2: Point
  target: Point
  delay: number
  duration: number
  size: number
  seed: number
  perchAngle: number
}

export interface FloralBorderOptions {
  seed: number
  margin: number
  light: boolean
  reducedMotion: boolean
  onDone: () => void
}

function clamp(value: number, lo: number, hi: number): number {
  return value < lo ? lo : value > hi ? hi : value
}

function makeRandom(seed: number): () => number {
  let state = seed >>> 0
  return () => {
    state = (state + 0x6d2b79f5) >>> 0
    let t = state
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

function rgba(color: Rgb, alpha: number): string {
  return `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${alpha})`
}

function easeOutBack(t: number): number {
  const c = 1.70158
  const k = t - 1
  return 1 + (c + 1) * k ** 3 + c * k ** 2
}

function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t ** 3 : 1 - (-2 * t + 2) ** 3 / 2
}

export function isLightPanel(hex: string): boolean {
  const match = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
  if (!match) return false
  const value = Number.parseInt(match[1], 16)
  return 0.2126 * ((value >> 16) & 255) + 0.7152 * ((value >> 8) & 255) + 0.0722 * (value & 255) > 150
}

function buildFrame(left: number, top: number, width: number, height: number, radius: number): Frame {
  const points: Point[] = []
  const line = (x0: number, y0: number, x1: number, y1: number) => {
    const steps = Math.max(1, Math.ceil(Math.hypot(x1 - x0, y1 - y0)))
    for (let i = 0; i < steps; i++) {
      const t = i / steps
      points.push({ x: x0 + (x1 - x0) * t, y: y0 + (y1 - y0) * t })
    }
  }
  const arc = (cx: number, cy: number, r: number, from: number, to: number) => {
    const steps = Math.max(1, Math.ceil(Math.abs(to - from) * r))
    for (let i = 0; i < steps; i++) {
      const angle = from + ((to - from) * i) / steps
      points.push({ x: cx + Math.cos(angle) * r, y: cy + Math.sin(angle) * r })
    }
  }

  const right = left + width
  const bottom = top + height
  const r = Math.max(0, Math.min(radius, width / 2, height / 2))

  line(left + width / 2, bottom, right - r, bottom)
  arc(right - r, bottom - r, r, Math.PI / 2, 0)
  line(right, bottom - r, right, top + r)
  arc(right - r, top + r, r, 0, -Math.PI / 2)
  line(right - r, top, left + r, top)
  arc(left + r, top + r, r, -Math.PI / 2, -Math.PI)
  line(left, top + r, left, bottom - r)
  arc(left + r, bottom - r, r, Math.PI, Math.PI / 2)
  line(left + r, bottom, left + width / 2, bottom)

  const cumulative: number[] = [0]
  for (let i = 1; i <= points.length; i++) {
    const a = points[i - 1]
    const b = points[i % points.length]
    cumulative.push(cumulative[i - 1] + Math.hypot(b.x - a.x, b.y - a.y))
  }
  const length = cumulative[cumulative.length - 1]

  const at = (u: number): FrameSample => {
    const target = ((u % length) + length) % length
    let lo = 0
    let hi = cumulative.length - 1
    while (lo + 1 < hi) {
      const mid = (lo + hi) >> 1
      if (cumulative[mid] <= target) lo = mid
      else hi = mid
    }
    const a = points[lo]
    const b = points[(lo + 1) % points.length]
    const span = cumulative[lo + 1] - cumulative[lo] || 1
    const t = (target - cumulative[lo]) / span
    return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, tx: (b.x - a.x) / span, ty: (b.y - a.y) / span }
  }

  return { length, at }
}

function buildGarland(width: number, height: number, margin: number, rand: () => number): Garland {
  const panelWidth = Math.max(1, width - margin * 2)
  const panelHeight = Math.max(1, height - margin * 2)
  const min = Math.min(panelWidth, panelHeight)
  const flowerSize = clamp(min * 0.09, 6.5, 12.5)
  const frame = buildFrame(margin, margin, panelWidth, panelHeight, 8)
  const half = frame.length / 2
  const leafSize = clamp(min * 0.16, 10, 22)
  const leafCount = clamp(Math.round(half / clamp(min * 0.18, 16, 30)), 8, 30)
  const flowerCount = clamp(Math.round(half / 240), 2, 5)

  const makeVine = (forward: boolean, delay: number): VineSpec => {
    const leaves: LeafSpec[] = []
    for (let i = 0; i < leafCount; i++) {
      leaves.push({
        f: clamp(0.05 + ((i + 0.5) / leafCount) * 0.9 + (rand() - 0.5) * 0.02, 0.03, 0.96),
        side: i % 2 === 0 ? 1 : -1,
        size: leafSize * (0.82 + rand() * 0.45),
        angle: 0.85 + rand() * 0.3,
        tendril: i % 5 === 2,
        seed: rand(),
      })
    }
    const flowers: FlowerSpec[] = []
    for (let i = 0; i < flowerCount; i++) {
      const base = flowerCount === 2 ? 0.28 + i * 0.34 : 0.16 + (i / (flowerCount - 1)) * 0.68
      flowers.push({
        f: clamp(base + (rand() - 0.5) * 0.04, 0.12, 0.85),
        size: flowerSize * (0.9 + rand() * 0.25),
        seed: rand(),
      })
    }
    const buds: BudSpec[] = [0.36, 0.6].map((base, i) => ({
      f: clamp(base + (rand() - 0.5) * 0.05, 0.15, 0.85),
      side: i % 2 === 0 ? -1 : 1,
      size: flowerSize * (0.5 + rand() * 0.18),
      seed: rand(),
    }))
    return {
      forward,
      delay,
      duration: GROW_MS,
      wavePhase: rand() * TAU,
      waveAmp: clamp(min * 0.013, 0.8, 2.4),
      waveLength: 90 + rand() * 70,
      leaves,
      flowers,
      buds,
    }
  }

  const vines = [makeVine(true, 80), makeVine(false, 260)]
  return {
    frame,
    vines,
    panelMin: min,
    tipSize: flowerSize * 1.4,
    tipSeed: rand(),
    tipAt: Math.max(...vines.map((vine) => timeAtProgress(vine, 1))) + 120,
  }
}

function buildVisitors(garland: Garland, width: number, height: number, rand: () => number): Visitor[] {
  return garland.vines.map((vine, index) => {
    const flower = vine.flowers[Math.min(1, vine.flowers.length - 1)]
    const p = vinePoint(garland.frame, vine, flower.f)
    const ox = -p.dy
    const oy = p.dx
    const target = { x: p.x + ox * flower.size * 0.8, y: p.y + oy * flower.size * 0.8 }
    const left = index === 0
    return {
      start: { x: left ? -26 : width + 26, y: height * (left ? 0.22 : 0.34) },
      c1: { x: width * (left ? 0.18 : 0.82), y: -height * 0.12 },
      c2: { x: target.x + width * (left ? -0.12 : 0.12), y: target.y - height * 0.35 },
      target,
      delay: FLY_AT + rand() * 260,
      duration: FLY_MS,
      size: clamp(garland.panelMin * 0.09, 6, 10),
      seed: rand(),
      perchAngle: Math.atan2(oy, ox) + Math.PI / 2,
    }
  })
}

function vinePoint(frame: Frame, vine: VineSpec, f: number): VineSample {
  const half = frame.length / 2
  const u = vine.forward ? f * half : frame.length - f * half
  const sample = frame.at(u)
  const wave = Math.sin(((f * half) / vine.waveLength) * TAU + vine.wavePhase) * vine.waveAmp * Math.sin(Math.PI * f)
  const sign = vine.forward ? 1 : -1
  return {
    x: sample.x - sample.ty * wave,
    y: sample.y + sample.tx * wave,
    dx: sample.tx * sign,
    dy: sample.ty * sign,
  }
}

function vineProgress(vine: VineSpec, elapsed: number): number {
  const x = clamp((elapsed - vine.delay) / vine.duration, 0, 1)
  return (1 - Math.cos(Math.PI * x)) / 2
}

function timeAtProgress(vine: VineSpec, f: number): number {
  return vine.delay + (vine.duration * Math.acos(1 - 2 * f)) / Math.PI
}

function cubicAt(a: Point, b: Point, c: Point, d: Point, t: number): Point {
  const u = 1 - t
  return {
    x: u ** 3 * a.x + 3 * u ** 2 * t * b.x + 3 * u * t ** 2 * c.x + t ** 3 * d.x,
    y: u ** 3 * a.y + 3 * u ** 2 * t * b.y + 3 * u * t ** 2 * c.y + t ** 3 * d.y,
  }
}

function drawPetal(ctx: CanvasRenderingContext2D, length: number, width: number, depth: number, palette: Palette) {
  const w = length * width
  const base = w * 0.28
  const path = new Path2D()
  path.moveTo(-base, 0)
  path.bezierCurveTo(-w, length * 0.16, -w * 0.98, length * 0.6, -w * 0.36, length * 0.84)
  path.bezierCurveTo(-w * 0.16, length * 0.97, w * 0.16, length * 0.97, w * 0.36, length * 0.84)
  path.bezierCurveTo(w * 0.98, length * 0.6, w, length * 0.16, base, 0)
  path.closePath()

  ctx.fillStyle = rgba(palette.petalMid, 0.24 * depth)
  ctx.fill(path)
  ctx.save()
  ctx.translate(0, length * 0.08)
  ctx.scale(0.7, 0.72)
  ctx.fillStyle = rgba(palette.petalLight, 0.18 * depth)
  ctx.fill(path)
  ctx.restore()

  ctx.strokeStyle = rgba(palette.petalDeep, 0.38 * depth)
  ctx.lineWidth = 0.55
  ctx.stroke(path)
}

function drawFlower(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  size: number,
  seed: number,
  open: number,
  palette: Palette,
  elapsed: number,
) {
  if (open <= 0.01) return
  const turn = seed * TAU * 0.15 + Math.sin(elapsed * 0.00045 + seed * TAU) * 0.06
  ctx.save()
  ctx.translate(x, y)
  ctx.rotate(turn)

  const rings = [
    { count: 5, len: 1, width: 0.6, offset: 0, depth: 1, delay: 0 },
    { count: 5, len: 0.64, width: 0.64, offset: Math.PI / 5, depth: 0.85, delay: 0.26 },
  ]
  for (const ring of rings) {
    for (let i = 0; i < ring.count; i++) {
      const petalOpen = clamp((open - ring.delay - (i / ring.count) * 0.32) / 0.62, 0, 1)
      if (petalOpen <= 0) continue
      const grow = Math.max(0.05, 0.2 + 0.8 * easeOutBack(petalOpen))
      ctx.save()
      ctx.rotate(ring.offset + (i / ring.count) * TAU)
      ctx.scale(grow, grow)
      drawPetal(ctx, size * ring.len, ring.width, ring.depth, palette)
      ctx.restore()
    }
  }

  const centerOpen = clamp((open - 0.4) / 0.5, 0, 1)
  if (centerOpen > 0) {
    ctx.save()
    ctx.scale(centerOpen, centerOpen)
    ctx.beginPath()
    ctx.arc(0, 0, size * 0.23, 0, TAU)
    ctx.fillStyle = rgba(palette.pollen, 0.55)
    ctx.fill()
    ctx.beginPath()
    ctx.arc(0, 0, size * 0.11, 0, TAU)
    ctx.fillStyle = rgba(palette.pollenLight, 0.6)
    ctx.fill()
    for (let i = 0; i < 7; i++) {
      const angle = (i / 7) * TAU + seed * TAU
      ctx.beginPath()
      ctx.arc(Math.cos(angle) * size * 0.31, Math.sin(angle) * size * 0.31, Math.max(0.55, size * 0.028), 0, TAU)
      ctx.fillStyle = rgba(palette.pollenLight, 0.75)
      ctx.fill()
    }
    ctx.restore()
  }
  ctx.restore()
}

function drawLeaf(
  ctx: CanvasRenderingContext2D,
  p: VineSample,
  leaf: LeafSpec,
  open: number,
  palette: Palette,
  elapsed: number,
) {
  if (open <= 0.01) return
  const outward = Math.atan2(p.dx, -p.dy)
  const angle = outward + leaf.side * leaf.angle + Math.sin(elapsed * 0.0006 + leaf.seed * TAU) * 0.02
  const length = leaf.size
  const w = length * 0.22
  ctx.save()
  ctx.translate(p.x, p.y)
  ctx.rotate(angle + Math.PI / 2)
  ctx.scale(0.3 + 0.7 * open, open)

  const path = new Path2D()
  path.moveTo(0, 0)
  path.bezierCurveTo(-w, -length * 0.16, -w * 0.74, -length * 0.62, 0, -length)
  path.bezierCurveTo(w * 0.74, -length * 0.62, w, -length * 0.16, 0, 0)
  ctx.fillStyle = rgba(palette.leaf, 0.22)
  ctx.fill(path)
  ctx.save()
  ctx.translate(0, -length * 0.04)
  ctx.scale(0.9, 0.94)
  ctx.fillStyle = rgba(palette.leafLight, 0.14)
  ctx.fill(path)
  ctx.restore()
  ctx.strokeStyle = rgba(palette.leafDeep, 0.45)
  ctx.lineWidth = 0.5
  ctx.stroke(path)

  ctx.beginPath()
  ctx.moveTo(0, -length * 0.06)
  ctx.lineTo(0, -length * 0.94)
  ctx.strokeStyle = rgba(palette.leafDeep, 0.42)
  ctx.lineWidth = 0.45
  ctx.stroke()
  ctx.restore()
}

function drawBud(ctx: CanvasRenderingContext2D, p: VineSample, bud: BudSpec, open: number, palette: Palette) {
  if (open <= 0.01) return
  const outward = Math.atan2(p.dx, -p.dy)
  const length = bud.size
  ctx.save()
  ctx.translate(p.x, p.y)
  ctx.rotate(outward + bud.side * 0.7 + Math.PI / 2)
  ctx.scale(0.5 + 0.5 * open, 0.5 + 0.5 * open)

  const path = new Path2D()
  path.moveTo(0, 0)
  path.bezierCurveTo(-length * 0.24, -length * 0.28, -length * 0.2, -length * 0.72, 0, -length)
  path.bezierCurveTo(length * 0.2, -length * 0.72, length * 0.24, -length * 0.28, 0, 0)
  ctx.fillStyle = rgba(palette.leaf, 0.22)
  ctx.fill(path)
  ctx.strokeStyle = rgba(palette.leafDeep, 0.45)
  ctx.lineWidth = 0.45
  ctx.stroke(path)

  ctx.beginPath()
  ctx.moveTo(-length * 0.18, -length * 0.18)
  ctx.quadraticCurveTo(-length * 0.05, -length * 0.45, 0, -length * 0.58)
  ctx.moveTo(length * 0.18, -length * 0.18)
  ctx.quadraticCurveTo(length * 0.05, -length * 0.45, 0, -length * 0.58)
  ctx.strokeStyle = rgba(palette.leafDeep, 0.3)
  ctx.stroke()

  if (open > 0.55) {
    const k = (open - 0.55) / 0.45
    ctx.beginPath()
    ctx.arc(0, -length * 0.98, length * 0.09 * k, 0, TAU)
    ctx.fillStyle = rgba(palette.pollen, 0.5 * k)
    ctx.fill()
  }
  ctx.restore()
}

function drawTendril(ctx: CanvasRenderingContext2D, p: VineSample, leaf: LeafSpec, open: number, palette: Palette) {
  const outward = Math.atan2(p.dx, -p.dy)
  const turns = 1.4 + leaf.seed * 0.7
  const span = turns * TAU * clamp(open, 0, 1)
  const size = leaf.size * 0.85
  ctx.save()
  ctx.translate(p.x, p.y)
  ctx.rotate(outward + leaf.side * 1.15)
  ctx.beginPath()
  let started = false
  for (let t = 0; t <= span; t += 0.1) {
    const f = t / (turns * TAU)
    const r = size * (0.06 + 0.64 * f ** 1.25)
    const x = Math.cos(t + leaf.seed * 6) * r
    const y = Math.sin(t + leaf.seed * 6) * r
    if (started) ctx.lineTo(x, y)
    else {
      ctx.moveTo(x, y)
      started = true
    }
  }
  ctx.strokeStyle = rgba(palette.stem, 0.5)
  ctx.lineWidth = 0.6
  ctx.lineCap = 'round'
  ctx.stroke()
  ctx.restore()
}

function drawStem(ctx: CanvasRenderingContext2D, frame: Frame, vine: VineSpec, progress: number, palette: Palette) {
  if (progress <= 0.001) return
  const half = frame.length / 2
  const steps = Math.max(2, Math.round((half * progress) / 1.5))
  const path = new Path2D()
  for (let i = 0; i <= steps; i++) {
    const p = vinePoint(frame, vine, progress * (i / steps))
    if (i === 0) path.moveTo(p.x, p.y)
    else path.lineTo(p.x, p.y)
  }
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'
  ctx.strokeStyle = rgba(palette.stem, 0.18)
  ctx.lineWidth = 4.2
  ctx.stroke(path)
  ctx.strokeStyle = rgba(palette.stem, 0.62)
  ctx.lineWidth = 1.7
  ctx.stroke(path)
  ctx.strokeStyle = rgba(palette.stemLight, 0.5)
  ctx.lineWidth = 0.6
  ctx.stroke(path)
}

function drawWing(ctx: CanvasRenderingContext2D, size: number, palette: Palette, upper: boolean) {
  ctx.beginPath()
  if (upper) {
    ctx.moveTo(0, -size * 0.16)
    ctx.bezierCurveTo(-size * 0.5, -size * 0.92, -size * 1.2, -size * 0.55, -size * 0.98, size * 0.02)
    ctx.bezierCurveTo(-size * 0.82, size * 0.3, -size * 0.32, size * 0.26, -size * 0.06, size * 0.12)
  } else {
    ctx.moveTo(-size * 0.05, size * 0.14)
    ctx.bezierCurveTo(-size * 0.62, size * 0.28, -size * 0.78, size * 0.82, -size * 0.38, size * 0.72)
    ctx.bezierCurveTo(-size * 0.16, size * 0.66, -size * 0.03, size * 0.46, 0, size * 0.34)
  }
  ctx.closePath()
  ctx.fillStyle = rgba(palette.petalMid, 0.2)
  ctx.fill()
  ctx.strokeStyle = rgba(palette.wing, 0.7)
  ctx.lineWidth = 0.55
  ctx.stroke()
  if (upper) {
    ctx.beginPath()
    ctx.moveTo(0, 0)
    ctx.quadraticCurveTo(-size * 0.45, -size * 0.2, -size * 0.88, size * 0.02)
    ctx.strokeStyle = rgba(palette.wing, 0.45)
    ctx.lineWidth = 0.4
    ctx.stroke()
  }
}

function drawButterfly(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  angle: number,
  size: number,
  flap: number,
  palette: Palette,
) {
  ctx.save()
  ctx.translate(x, y)
  ctx.rotate(angle)
  ctx.save()
  ctx.scale(0.18 + 0.82 * flap, 1)
  for (const side of [-1, 1]) {
    ctx.save()
    ctx.scale(side, 1)
    drawWing(ctx, size, palette, true)
    drawWing(ctx, size, palette, false)
    ctx.beginPath()
    ctx.arc(-size * 0.58, -size * 0.32, Math.max(0.45, size * 0.07), 0, TAU)
    ctx.fillStyle = rgba(palette.petalLight, 0.4)
    ctx.fill()
    ctx.restore()
  }
  ctx.restore()

  ctx.strokeStyle = rgba(palette.wingEdge, 0.75)
  ctx.lineWidth = Math.max(0.6, size * 0.09)
  ctx.lineCap = 'round'
  ctx.beginPath()
  ctx.moveTo(0, -size * 0.32)
  ctx.lineTo(0, size * 0.55)
  ctx.stroke()
  ctx.beginPath()
  ctx.arc(0, -size * 0.38, Math.max(0.7, size * 0.1), 0, TAU)
  ctx.fillStyle = rgba(palette.wingEdge, 0.8)
  ctx.fill()
  ctx.strokeStyle = rgba(palette.wingEdge, 0.55)
  ctx.lineWidth = 0.45
  for (const side of [-1, 1]) {
    ctx.beginPath()
    ctx.moveTo(0, -size * 0.42)
    ctx.quadraticCurveTo(side * size * 0.3, -size * 0.75, side * size * 0.46, -size * 0.86)
    ctx.stroke()
  }
  ctx.restore()
}

function visitorPose(visitor: Visitor, elapsed: number, still: boolean) {
  if (still) return { x: visitor.target.x, y: visitor.target.y, angle: visitor.perchAngle, flap: 0.7 }
  const raw = clamp((elapsed - visitor.delay) / visitor.duration, 0, 1)
  if (raw < 1) {
    const eased = easeInOutCubic(raw)
    const p = cubicAt(visitor.start, visitor.c1, visitor.c2, visitor.target, eased)
    const ahead = cubicAt(visitor.start, visitor.c1, visitor.c2, visitor.target, Math.min(1, eased + 0.01))
    const dir = Math.atan2(ahead.y - p.y, ahead.x - p.x)
    const wobble = Math.sin(elapsed * 0.018 + visitor.seed * 8) * 2.4 * (1 - raw)
    return {
      x: p.x - Math.sin(dir) * wobble,
      y: p.y + Math.cos(dir) * wobble,
      angle: dir + Math.PI / 2,
      flap: 0.25 + 0.75 * Math.abs(Math.sin(elapsed * 0.05 + visitor.seed * 9)),
    }
  }
  const bob = Math.sin(elapsed * 0.0016 + visitor.seed * 7)
  return {
    x: visitor.target.x + Math.cos(visitor.seed * TAU) * bob * 1.1,
    y: visitor.target.y + Math.sin(visitor.seed * TAU) * bob * 1.1 + Math.sin(elapsed * 0.002 + visitor.seed * 3) * 0.8,
    angle: visitor.perchAngle + Math.sin(elapsed * 0.0011 + visitor.seed * 5) * 0.09,
    flap: 0.35 + 0.6 * Math.abs(Math.sin(elapsed * 0.0055 + visitor.seed * 8)),
  }
}

function drawScene(
  ctx: CanvasRenderingContext2D,
  garland: Garland,
  visitors: Visitor[],
  elapsed: number,
  still: boolean,
  palette: Palette,
) {
  for (const vine of garland.vines) {
    drawStem(ctx, garland.frame, vine, still ? 1 : vineProgress(vine, elapsed), palette)
  }

  for (const vine of garland.vines) {
    const progress = still ? 1 : vineProgress(vine, elapsed)
    for (const leaf of vine.leaves) {
      if (leaf.f > progress) continue
      const open = still ? 1 : clamp((elapsed - timeAtProgress(vine, leaf.f)) / LEAF_MS, 0, 1)
      drawLeaf(ctx, vinePoint(garland.frame, vine, leaf.f), leaf, open, palette, elapsed)
    }
    for (const leaf of vine.leaves) {
      if (!leaf.tendril || leaf.f > progress) continue
      const open = still ? 1 : clamp((elapsed - timeAtProgress(vine, leaf.f) - LEAF_MS * 0.5) / 700, 0, 1)
      drawTendril(ctx, vinePoint(garland.frame, vine, leaf.f), leaf, open, palette)
    }
    for (const bud of vine.buds) {
      if (bud.f > progress) continue
      const open = still ? 1 : clamp((elapsed - timeAtProgress(vine, bud.f)) / 700, 0, 1)
      drawBud(ctx, vinePoint(garland.frame, vine, bud.f), bud, open, palette)
    }
    for (const flower of vine.flowers) {
      if (flower.f > progress) continue
      const open = still ? 1 : clamp((elapsed - timeAtProgress(vine, flower.f) + 40) / FLOWER_MS, 0, 1)
      const p = vinePoint(garland.frame, vine, flower.f)
      drawFlower(ctx, p.x, p.y, flower.size, flower.seed, open, palette, elapsed)
    }
  }

  const tip = garland.frame.at(garland.frame.length / 2)
  const tipOpen = still ? 1 : clamp((elapsed - garland.tipAt) / (FLOWER_MS * 1.4), 0, 1)
  drawFlower(ctx, tip.x, tip.y, garland.tipSize, garland.tipSeed, tipOpen, palette, elapsed)

  for (const visitor of visitors) {
    const pose = visitorPose(visitor, elapsed, still)
    drawButterfly(ctx, pose.x, pose.y, pose.angle, visitor.size, pose.flap, palette)
  }
}

export function runFloralBorder(canvas: HTMLCanvasElement, options: FloralBorderOptions): () => void {
  const ctx = canvas.getContext('2d')
  if (!ctx) return () => {}

  const palette = options.light ? PAPER_INK : NIGHT_INK
  const seed = options.seed || 1
  const fadeAt = options.reducedMotion ? FADE_AT_STILL : FADE_AT
  const endAt = fadeAt + FADE_MS
  const started = performance.now()

  let width = 0
  let height = 0
  let ratio = 0
  let garland: Garland | null = null
  let visitors: Visitor[] = []
  let staticLayer: HTMLCanvasElement | null = null
  let staticAt = 0
  let raf = 0
  let done = false

  const layout = () => {
    const w = Math.max(1, canvas.clientWidth || window.innerWidth)
    const h = Math.max(1, canvas.clientHeight || window.innerHeight)
    const dpr = Math.min(1.5, window.devicePixelRatio || 1)
    if (garland && w === width && h === height && dpr === ratio) return
    width = w
    height = h
    ratio = dpr
    canvas.width = Math.max(1, Math.round(w * dpr))
    canvas.height = Math.max(1, Math.round(h * dpr))
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    garland = buildGarland(w, h, options.margin, makeRandom(seed))
    visitors = buildVisitors(garland, w, h, makeRandom(seed ^ 0x9e3779b9))
    staticLayer = null
    staticAt = options.reducedMotion ? 0 : garland.tipAt + FLOWER_MS * 1.4
  }

  const frame = (now: number) => {
    layout()
    const elapsed = now - started
    const alpha = elapsed > fadeAt ? Math.max(0, 1 - (elapsed - fadeAt) / FADE_MS) : 1
    ctx.clearRect(0, 0, width, height)
    if (garland && alpha > 0) {
      ctx.save()
      ctx.globalAlpha = alpha
      if (elapsed >= staticAt) {
        if (!staticLayer) {
          staticLayer = document.createElement('canvas')
          staticLayer.width = Math.round(width * ratio)
          staticLayer.height = Math.round(height * ratio)
          const layerCtx = staticLayer.getContext('2d')
          if (layerCtx) {
            layerCtx.setTransform(ratio, 0, 0, ratio, 0, 0)
            drawScene(layerCtx, garland, [], elapsed, true, palette)
          }
        }
        ctx.drawImage(staticLayer, 0, 0, width, height)
        for (const visitor of visitors) {
          const pose = visitorPose(visitor, elapsed, options.reducedMotion)
          drawButterfly(ctx, pose.x, pose.y, pose.angle, visitor.size, pose.flap, palette)
        }
      } else {
        drawScene(ctx, garland, visitors, elapsed, options.reducedMotion, palette)
      }
      ctx.restore()
    }
    if (elapsed >= endAt) {
      if (!done) {
        done = true
        options.onDone()
      }
      return
    }
    raf = requestAnimationFrame(frame)
  }

  raf = requestAnimationFrame(frame)
  return () => cancelAnimationFrame(raf)
}
