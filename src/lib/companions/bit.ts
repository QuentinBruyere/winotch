import {
  compose,
  hold,
  layer,
  moved,
  times,
  type Companion,
  type Frame,
  type Layer,
} from './companion'

// Bit (DF-0020): a little robot, 24 × 16 pixels, standing in the middle. A
// head with a screen for a face (its eyes light up in the screen), an
// antenna with a bulb, two feet; arms only while working, and one raised
// for a question. Each picture is put together from the parts below;
// `pose` draws the robot 2 pixels lower than they say, under the symbols
// above its head.
//
// What it does, by state (★ = Bit's own, the others shared with every
// companion so far):
//
// | State     | Arrival (once)              | Loop                                        |
// |-----------|-----------------------------|---------------------------------------------|
// | neutral   | blinks, looks around, eyes  | ★ sleep mode (eyes shut, "z"s rising)       |
// |           | closing                     |                                             |
// | active    | -                           | types (★ code scrolling on its screen,      |
// |           |                             | bulb blinking), ★ radio waves from the      |
// |           |                             | antenna, ★ loading bar on the screen,       |
// |           |                             | ★ walks side to side, ★ scans with its eyes |
// | attention | -                           | blinking "!", shakes                        |
// | question  | -                           | raises an arm under a "?", looks up, left,  |
// |           |                             | right                                       |
// | success   | ★ check mark on the screen  | still, ★ happy eyes, bulb on                |
// | error     | ★ reboots: snow, black      | crossed eyes, ★ a spark now and then        |
//
// Moods (the Companion module, DF-0024): awake and calm by day (idle,
// ★ scans), dances with notes and a flashing
// bulb, runs on the spot, watches you, petted (★ a heart on its screen),
// stretches its arms; alarm = attention, sleep = neutral.

const WIDTH = 24
const HEIGHT = 16

// The head at (6, 4): rows 4 to 11, the screen a hole from (8, 6) to
// (15, 9).
const HEAD = [
  '.##########.',
  '############',
  '##oooooooo##',
  '##oooooooo##',
  '##oooooooo##',
  '##oooooooo##',
  '############',
  '.##########.',
]
// Feet standing, and the two steps of a walk.
const FEET = {
  stand: layer(['.##....##.'], 7, 12),
  stepLeft: layer(['##.....##.'], 7, 12),
  stepRight: layer(['.##.....##'], 7, 12),
}
type Feet = keyof typeof FEET
const STEM = layer(['#'], 11, 3)
const BULB = {
  on: layer(['.#.', '###'], 10, 1),
  off: layer(['#'], 11, 2),
}
type Bulb = keyof typeof BULB

// Eyes, lit in the screen: the left one at (x, y), the right one 4 pixels
// further.
const EYES = {
  open: ['##', '##'],
  half: ['..', '##'],
  closed: ['..', '..'],
  happy: ['.#.', '#.#'],
  cross: ['#.#', '.#.', '#.#'],
}
type Eyes = keyof typeof EYES

// Arms: only while working, and one raised for a question; none otherwise.
const ARMS: Record<string, Layer[]> = {
  none: [],
  // The right one raised, as to ask.
  raise: [layer(['.#', '.#', '#.'], 18, 4)],
  up: [layer(['#.', '.#'], 4, 6), layer(['.#', '#.'], 18, 6)],
  out: [layer(['##'], 4, 8), layer(['##'], 18, 8)],
  // Typing: one arm forward, then the other.
  leftOut: [layer(['##'], 4, 8), layer(['#.', '.#'], 18, 8)],
  rightOut: [layer(['.#', '#.'], 4, 8), layer(['##'], 18, 8)],
}
type Arms = 'none' | 'raise' | 'up' | 'out' | 'leftOut' | 'rightOut'

// Above the head, drawn where they are (the robot stands 2 pixels lower).
const BANG = layer(['##', '##', '..', '##'], 17, 0)
const QUESTION = layer(['###', '..#', '.##', '...', '.#.'], 16, 0)
const ZZ = ['###', '.#.', '###']
const SPARK = layer(['.#.', '#.#', '.#.'], 14, 0)
// Radio waves on each side of the bulb (rows 3 and 4 once lowered).
const WAVES = {
  near: [layer(['.#', '#.', '.#'], 8, 2), layer(['#.', '.#', '#.'], 13, 2)],
  far: [layer(['.#', '#.', '#.', '.#'], 6, 1), layer(['#.', '.#', '.#', '#.'], 16, 1)],
}

// What the screen can show instead of the eyes, at its top-left (8, 6).
const screen = (rows: string[]) => [layer(rows, 8, 6)]
const CODE = ['###.##..', '##.####.', '#####...', '.###.##.', '####.#..']
const code = (step: number) => [
  layer([CODE[step % CODE.length]], 8, 6),
  layer([CODE[(step + 1) % CODE.length]], 8, 8),
]
const CHECK = ['......##', '##...##.', '.##.##..', '..###...']
const bar = (filled: number) => screen(['........', '#'.repeat(filled), '#'.repeat(filled)])

type Pose = {
  // The whole robot moved by (dx, dy).
  dx?: number
  dy?: number
  eyes?: Eyes
  // Where the eyes look, in pixels from the middle.
  look?: [number, number]
  arms?: Arms
  bulb?: Bulb
  feet?: Feet
  // What the screen shows instead of the eyes.
  screen?: Layer[]
  // Drawn as they are, not moved with the robot.
  extra?: Layer[]
}

// The robot in a pose.
function pose(p: Pose): Frame {
  const { dx = 0, eyes = 'open', look = [0, 0], arms = 'none', bulb = 'off' } = p
  const dy = (p.dy ?? 0) + 2
  const move = (l: Layer) => moved(l, dx, dy)
  const [lx, ly] = look
  // Wide eyes sit one pixel more to the side, crossed ones one higher.
  const wide = EYES[eyes][0].length === 3 ? 1 : 0
  const up = eyes === 'cross' ? 1 : 0
  const face = p.screen ?? [
    layer(EYES[eyes], 9 - wide + lx, 7 + ly - up),
    layer(EYES[eyes], 13 + lx, 7 + ly - up),
  ]
  return compose(
    WIDTH,
    HEIGHT,
    move(layer(HEAD, 6, 4)),
    move(FEET[p.feet ?? 'stand']),
    move(STEM),
    move(BULB[bulb]),
    ...face.map(move),
    ...ARMS[arms].map(move),
    ...(p.extra ?? []),
  )
}

const rest = pose({})
const blink = (['half', 'closed', 'half'] as const).map((eyes) => pose({ eyes }))

// --- Resting ---

// Sleep mode: eyes shut, a "z" rising.
const sleep = [
  ...hold(pose({ eyes: 'closed', extra: [layer(ZZ, 16, 2)] }), 4),
  ...hold(pose({ eyes: 'closed', extra: [layer(ZZ, 17, 0)] }), 4),
  ...hold(pose({ eyes: 'closed' }), 4),
]

// --- Working ---

// Typing with code on the screen, the bulb blinking.
const typing = (steps: number) =>
  Array.from({ length: steps }, (_, i) =>
    pose({
      arms: i % 2 ? 'rightOut' : 'leftOut',
      bulb: Math.floor(i / 2) % 2 ? 'on' : 'off',
      screen: code(Math.floor(i / 2)),
    }),
  )
// Radio waves from the antenna.
const radio = times(
  [
    pose({ bulb: 'on', look: [0, -1] }),
    pose({ bulb: 'on', look: [0, -1], extra: WAVES.near }),
    pose({ bulb: 'on', look: [0, -1], extra: [...WAVES.near, ...WAVES.far] }),
    pose({ bulb: 'on', look: [0, -1], extra: WAVES.far }),
    pose({ look: [0, -1] }),
  ],
  3,
)
// A loading bar fills the screen, blinks once full.
const loading = [
  ...Array.from({ length: 8 }, (_, i) => pose({ bulb: 'on', screen: bar(i + 1) })),
  ...times([pose({ bulb: 'on', screen: bar(8) }), pose({ bulb: 'on', screen: bar(0) })], 2),
  ...hold(pose({ bulb: 'on' }), 2),
]
// Walks a few steps one way, one pixel every second picture.
const walking = (from: number, to: number) => {
  const step = to > from ? 1 : -1
  return Array.from({ length: Math.abs(to - from) * 2 }, (_, i) =>
    pose({
      dx: from + step * Math.floor((i + 1) / 2),
      look: [step, 0],
      feet: i % 2 ? 'stepRight' : 'stepLeft',
    }),
  )
}
const walk = [
  ...walking(0, 4),
  ...hold(pose({ dx: 4 }), 3),
  ...walking(4, -4),
  ...hold(pose({ dx: -4 }), 3),
  ...walking(-4, 0),
  ...hold(rest, 2),
]
// Scanning: the eyes sweep from left to right and back.
const scan = [
  ...hold(pose({ look: [-1, 0], bulb: 'on' }), 2),
  ...hold(pose({ bulb: 'on' }), 2),
  ...hold(pose({ look: [1, 0], bulb: 'on' }), 2),
  ...hold(pose({ bulb: 'on' }), 2),
]

// --- Asking ---

// The bulb on, under a "!" or not.
const calling = (bang: boolean) => pose({ bulb: 'on', extra: bang ? [BANG] : [] })
const shaking = (dx: number) => pose({ dx, bulb: 'on' })
// An arm raised, under a question mark.
const wondering = (look: [number, number], eyes: Eyes = 'open') =>
  pose({ look, eyes, arms: 'raise', bulb: 'on', extra: [QUESTION] })

// --- Done and failed ---

// Snow on the screen: the same pixels every time for a given step.
const snow = (step: number) =>
  screen(
    Array.from({ length: 4 }, (_, y) =>
      Array.from({ length: 8 }, (_, x) => ((x * 7 + y * 13 + step * 5) % 3 === 0 ? '#' : '.')).join(
        '',
      ),
    ),
  )

// --- Moods (the Companion module, DF-0024) ---

const NOTE = ['.##', '.#.', '##.']
const note = (x: number, y: number) => layer(NOTE, x, y)
// Speed lines behind a runner.
const lines = (y: number) => [layer(['###', '...', '.##'], 0, y)]
const HEART = ['#.#', '###', '.#.']

// Dances: bobs, arms up, the bulb flashing, notes around.
const dance = times(
  [
    ...hold(pose({ dy: -1, arms: 'up', bulb: 'on', extra: [note(2, 3)] }), 2),
    ...hold(pose({ arms: 'out', extra: [note(2, 5)] }), 2),
    ...hold(pose({ dy: -1, arms: 'up', bulb: 'on', extra: [note(20, 3)] }), 2),
    ...hold(pose({ arms: 'out', extra: [note(20, 5)] }), 2),
  ],
  2,
)
// Runs on the spot: feet and arms swapping.
const run = times(
  [
    pose({ look: [1, 0], feet: 'stepLeft', arms: 'leftOut', extra: lines(9) }),
    pose({ dy: -1, look: [1, 0], feet: 'stepRight', arms: 'rightOut', extra: lines(10) }),
  ],
  4,
)
// Awake, bulb on, looking at you.
const watch = [
  ...hold(pose({ bulb: 'on' }), 14),
  ...blink,
  ...hold(pose({ bulb: 'on', look: [0, -1] }), 6),
]
// Petted: a heart on its screen.
const HEART_SCREEN = ['.##..##.', '########', '.######.', '..####..']
const petted = [
  ...hold(pose({ bulb: 'on', screen: screen(HEART_SCREEN) }), 6),
  pose({ bulb: 'on', screen: [] }),
  ...hold(pose({ bulb: 'on', screen: screen(HEART_SCREEN) }), 4),
]
// Stretches its arms up, eyes shut, the bulb flickering, then blinks.
const stretching = [
  rest,
  ...hold(pose({ arms: 'up', eyes: 'closed', bulb: 'on' }), 3),
  pose({ arms: 'up', eyes: 'closed' }),
  ...hold(pose({ arms: 'up', eyes: 'closed', bulb: 'on' }), 3),
  rest,
  ...blink,
]

export const bit: Companion = {
  width: WIDTH,
  height: HEIGHT,
  animations: {
    // Waiting for a prompt: blinks, looks around once, its eyes close, then
    // it stays in sleep mode ("z"s) as long as it waits.
    neutral: {
      fps: 8,
      intro: [
        ...hold(rest, 6),
        ...blink,
        ...hold(pose({ look: [-1, 0] }), 4),
        ...hold(pose({ look: [1, 0] }), 4),
        ...hold(pose({ eyes: 'half' }), 4),
      ],
      frames: sleep,
    },
    // Working: types with code on the screen, sends radio waves, loads,
    // walks about, scans.
    active: {
      fps: 8,
      frames: [
        ...typing(16),
        ...radio,
        ...typing(12),
        ...loading,
        ...walk,
        ...typing(12),
        ...times(scan, 2),
      ],
    },
    // A permission to give: a blinking "!", then shakes.
    attention: {
      fps: 8,
      frames: [
        ...times([...hold(calling(true), 3), ...hold(calling(false), 2)], 3),
        ...times([shaking(-1), shaking(1)], 4),
        ...hold(pose({ bulb: 'on' }), 4),
      ],
    },
    // Waiting for an answer: raises an arm under a "?", looks up, left,
    // right.
    question: {
      fps: 8,
      frames: [
        ...hold(wondering([0, -1]), 8),
        ...hold(wondering([-1, 0]), 6),
        ...hold(wondering([1, 0]), 6),
        ...hold(wondering([0, 0]), 6),
        ...(['half', 'closed', 'half'] as const).map((eyes) => wondering([0, 0], eyes)),
        ...hold(wondering([0, 0]), 4),
      ],
    },
    // Done: a check mark on the screen once, then happy eyes, still (a
    // moving success distracts, DF-0020).
    success: {
      fps: 8,
      intro: [
        ...hold(pose({ bulb: 'on', screen: screen(CHECK) }), 6),
        pose({ bulb: 'on', screen: [] }),
        ...hold(pose({ bulb: 'on', screen: screen(CHECK) }), 4),
      ],
      frames: [pose({ eyes: 'happy', bulb: 'on' })],
    },
    // Failed: reboots (snow on the screen, then black), then crossed eyes,
    // a spark now and then.
    error: {
      fps: 8,
      intro: [
        ...[0, 1, 2, 3, 4, 5].map((step) => pose({ screen: snow(step) })),
        ...hold(pose({ screen: [] }), 4),
      ],
      frames: [
        ...hold(pose({ eyes: 'cross', extra: [SPARK] }), 2),
        ...hold(pose({ eyes: 'cross' }), 12),
      ],
    },
  },
  moods: {
    // A quiet day: its scenes in a random order (scanning, radio waves, a
    // little walk, a nap in sleep mode…), a calm pause before each.
    idle: {
      fps: 8,
      frames: [...hold(rest, 20), ...blink, ...hold(rest, 8)],
      scenes: [
        [...hold(pose({ look: [-1, 0] }), 6), ...hold(pose({ look: [1, 0] }), 6)],
        scan,
        radio,
        [...hold(pose({ eyes: 'half' }), 3), ...times(sleep, 3), ...blink],
        walk,
      ],
    },
    dance: { fps: 8, frames: dance },
    run: { fps: 8, frames: run },
    watch: { fps: 8, frames: watch },
    petted: { fps: 8, intro: petted, frames: [pose({ eyes: 'happy', bulb: 'on' })] },
    stretch: { fps: 8, intro: stretching, frames: [rest] },
  },
}
