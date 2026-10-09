import type { Tone } from '../content'
import type { Mood } from './mood.svelte'

// A companion (DF-0020): a little pixel creature drawn instead of an item's
// dot, in the tone's color, with one animation per tone. An animation may
// open with an arrival played once when the tone starts (confetti, a fall),
// then loops several short scenes chained (a bounce, a look around…), so it
// does not feel repetitive. The success tone loops a single still picture
// (DF-0020): only its arrival moves.

// One picture: `height` rows of `width` characters, `.` = empty, `#` = a
// pixel in the companion's color; an imported companion with its own colors
// (DF-0025) uses other characters, named in its `palette`.
export type Frame = string[]

export type Animation = {
  // Pictures per second; every picture lasts the same time, a longer one is
  // repeated (`hold`).
  fps: number
  // Played once when the tone starts, before the loop.
  intro?: Frame[]
  // Played in a loop.
  frames: Frame[]
  // Scenes played in an order drawn at random each time the state starts,
  // `frames` (a calm pause) before each; the loop then repeats that order
  // (`played`).
  scenes?: Frame[][]
}

// Every frame is `width` × `height` pixels: wider than high (`RATIO`), the
// creature standing in the middle, the sides for its scenes (walking, a
// ball…).
export type Companion = {
  width: number
  height: number
  animations: Record<Tone, Animation>
  // For the Companion module (DF-0024): dancing, running, watching, petted,
  // stretching. A missing one borrows a state's animation (`moodAnimation`).
  moods: Partial<Record<Mood, Animation>>
  // An imported companion keeping its own colors (DF-0025): each character
  // of its pictures and its color.
  palette?: Record<string, string>
}

// A companion's width for its height (24 × 16).
export const RATIO = 1.5

// How much of its height each of its sides overlaps its neighbours: the
// room for its scenes is empty at rest. What it takes in a row is then
// `RATIO - 2 * OVERLAP` times its height. Enough is left for a little gap
// between two companions (Miso's tail reaches further right).
export const OVERLAP = 0.22

// A piece of a picture placed at (`x`, `y`): `#` paints a pixel, `o` clears
// one (an eye, a mouth), anything else leaves the picture as it is.
export type Layer = { pattern: string[]; x: number; y: number }

export const layer = (pattern: string[], x: number, y: number): Layer => ({ pattern, x, y })

// `l` moved by (`dx`, `dy`).
export const moved = (l: Layer, dx: number, dy: number): Layer =>
  layer(l.pattern, l.x + dx, l.y + dy)

// A `width` × `height` picture made of `layers`, painted in order.
export function compose(width: number, height: number, ...layers: Layer[]): Frame {
  const grid = Array.from({ length: height }, () => Array<string>(width).fill('.'))
  for (const { pattern, x, y } of layers) {
    pattern.forEach((row, dy) => {
      ;[...row].forEach((c, dx) => {
        const cell = grid[y + dy]
        if (cell === undefined || x + dx < 0 || x + dx >= width) return
        if (c === '#') cell[x + dx] = '#'
        else if (c === 'o') cell[x + dx] = '.'
      })
    })
  }
  return grid.map((row) => row.join(''))
}

// `frame` shown `count` times in a row: it lasts longer.
export const hold = (frame: Frame, count: number): Frame[] => Array<Frame>(count).fill(frame)

// `frames` played `count` times in a row.
export const times = (frames: Frame[], count: number): Frame[] =>
  Array.from({ length: count }, () => frames).flat()

// `frames` then the same backwards, without repeating the turning pictures.
export const thereAndBack = (frames: Frame[]): Frame[] => [
  ...frames,
  ...frames.slice(1, -1).reverse(),
]

// Confetti: `count` pixels falling from above the top while drifting, one
// layer list per picture, over `steps` pictures; the same every time (a
// fixed pseudo-random sequence).
export function confetti(width: number, count: number, steps: number): Layer[][] {
  let seed = 7
  const next = () => {
    seed = (seed * 1103515245 + 12345) % 2147483648
    return seed / 2147483648
  }
  const pieces = Array.from({ length: count }, () => ({
    x: Math.floor(next() * width),
    y: -Math.floor(next() * 8),
    drift: next() < 0.5 ? -1 : 1,
  }))
  return Array.from({ length: steps }, (_, step) =>
    pieces.map((p) => layer(['#'], p.x + (step % 4 >= 2 ? p.drift : 0), p.y + step * 2)),
  )
}

// The animation as played: with scenes, in an order drawn from `seed` (the
// same seed, the same order: a companion redrawn elsewhere goes on alike),
// each after the calm pause in `frames`.
export function played(animation: Animation, seed: number): Animation {
  if (!animation.scenes) return animation
  let state = Math.floor(seed) || 1
  const random = () => {
    // mulberry32: a small, good enough pseudo-random generator.
    state = (state + 0x6d2b79f5) | 0
    let t = Math.imul(state ^ (state >>> 15), 1 | state)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
  const order = [...animation.scenes]
  for (let i = order.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1))
    ;[order[i], order[j]] = [order[j], order[i]]
  }
  return {
    fps: animation.fps,
    intro: animation.intro,
    frames: order.flatMap((scene) => [...animation.frames, ...scene]),
  }
}

// One picture: an SVG path per character of its frame (`#`, or a palette
// color).
export type Picture = { char: string; d: string }[]

// An animation's distinct pictures, each as SVG paths (a rectangle per run
// of same pixels on a row, to keep it short), and the order they play in:
// the intro's first, then the loop's.
export function pictures(animation: Animation): { paths: Picture[]; order: number[] } {
  const paths: Picture[] = []
  const keys: string[] = []
  const order = [...(animation.intro ?? []), ...animation.frames].map((frame) => {
    const byChar = new Map<string, string>()
    frame.forEach((row, y) => {
      let x = 0
      while (x < row.length) {
        const char = row[x]
        if (char === '.') {
          x++
          continue
        }
        const start = x
        while (row[x] === char) x++
        byChar.set(char, (byChar.get(char) ?? '') + `M${start} ${y}h${x - start}v1h${start - x}z`)
      }
    })
    const picture = [...byChar.entries()]
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
      .map(([char, d]) => ({ char, d }))
    const key = picture.map((p) => p.char + p.d).join('|')
    const known = keys.indexOf(key)
    if (known !== -1) return known
    keys.push(key)
    paths.push(picture)
    return paths.length - 1
  })
  return { paths, order }
}

// A companion's animation for a mood: its own, else the closest state's.
export function moodAnimation(companion: Companion, mood: Mood): Animation {
  const own = companion.moods[mood]
  if (own) return own
  const { neutral, active, attention, success } = companion.animations
  const awake = neutral.intro?.[0] ?? neutral.frames[0]
  switch (mood) {
    case 'alarm':
      return attention
    case 'dance':
    case 'run':
      return active
    case 'petted':
      return success
    case 'watch':
    case 'stretch':
    case 'idle':
      return { fps: 1, frames: [awake] }
    case 'sleep':
      return neutral
  }
}

// The middle row of a companion standing awake (the first picture of its
// neutral animation, arrival included, without symbols above the head),
// antenna or feet included: it is drawn moved so this row sits centered on
// the texts, even while it sleeps lower.
export function center(companion: Companion): number {
  const neutral = companion.animations.neutral
  const frame = neutral.intro?.[0] ?? neutral.frames[0]
  const rows = frame.flatMap((row, y) => (/[^.]/.test(row) ? [y] : []))
  return rows.length === 0 ? companion.height / 2 : (rows[0] + rows[rows.length - 1] + 1) / 2
}
