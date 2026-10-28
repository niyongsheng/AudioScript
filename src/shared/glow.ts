const COLORS = [
  '#3186ff',
  '#9378ff',
  '#f96bd6',
  '#fc413d',
  '#fc413d',
  '#ff6b2b',
  '#fec700',
  '#ffdb0f',
  '#88de42',
  '#0ebc5f',
  '#0ebc5f',
  '#2eaab2',
  '#00a9bb',
  '#3186ff',
  '#3186ff',
]

function palette(stops: readonly number[]): string {
  return COLORS.slice(0, stops.length)
    .map((color, index) => `${color} ${stops[index]}%`)
    .join(', ')
}

export const GLOW_PALETTE = palette([34, 37, 39, 41, 48, 50, 52, 56, 58, 61, 65, 70, 72, 73, 100])

export const GLOW_RING_PALETTE = palette([0, 7.7, 12.8, 17.9, 35.9, 41, 46.2, 56.4, 61.5, 69.2, 79.5, 92.3, 97.4, 100])
