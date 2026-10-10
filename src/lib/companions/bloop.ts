import {
  compose,
  confetti,
  hold,
  layer,
  moved,
  times,
  type Companion,
  type Frame,
  type Layer,
} from './companion'

// Bloop (DF-0020): a little round drop with two eyes, 24 × 16 pixels, standing
// in the middle; arms only while working, and one scratching its head for a
// question.
// Each picture is put together from the parts below.
//
// What it does, by state (★ = Bloop's own, the others shared with every
// companion so far):
//
// | State     | Arrival (once)    | Loop                                                  |
// |-----------|-------------------|-------------------------------------------------------|
// | neutral   | blinks, looks     | ★ naps as a puddle ("z"s rising)                      |
// |           | around, ★ melts   |                                                       |
// |           | into a puddle     |                                                       |
// | active    | -                 | bounces arms up, types,                               |
// |           |                   | ★ rolls side to side (eyes going round),              |
// |           |                   | ★ wobbles like jelly, ★ bounces a ball on its head    |
// | attention | -                 | mouth open under a blinking "!", shakes               |
// | question  | -                 | scratches its head under a "?", looks up, left, right |
// | success   | ★ confetti falls  | still (no smile: the color says it)                   |
// | error     | ★ falls flat      | flat, crossed eyes, dazed now and then                |
//
// Moods (the Companion module, DF-0024): awake and calm by day (idle),
// dances with notes, runs on the
// spot, watches you, petted (hearts), ★ stretches tall; alarm = attention,
// sleep = neutral.

const WIDTH = 24
const HEIGHT = 16

// The drop, its top-left corner at (7, 5): rows 5 to 13, columns 7 to 16.
const BODY = [
  '..######..',
  '.########.',
  '##########',
  '##########',
  '##########',
  '##########',
  '##########',
  '.########.',
  '..######..',
]

// Landing from a bounce, or wobbling: wider and lower, at (6, 7).
const SQUASH = [
  '..########..',
  '.##########.',
  '############',
  '############',
  '############',
  '.##########.',
  '..########..',
]

// Wobbling: taller and thinner, at (8, 3).
const STRETCH = [
  '..####..',
  '.######.',
  '########',
  '########',
  '########',
  '########',
  '########',
  '########',
  '########',
  '.######.',
  '..####..',
]

// Knocked flat or melting, at (5, 9).
const FLAT = [
  '...########...',
  '.############.',
  '##############',
  '.############.',
  '...########...',
]

// Melted into a puddle, at (4, 11).
const PUDDLE = ['....########....', '.##############.', '################']

// Eyes: holes, the left one at (x, y), the right one 4 pixels further.
const EYES = {
  open: ['oo', 'oo'],
  half: ['..', 'oo'],
  closed: ['..', '..'],
}
type Eyes = keyof typeof EYES

// Arms: only while working, and one scratching the head for a question;
// none otherwise.
const ARMS: Record<string, Layer[]> = {
  none: [],
  // The right one up over the head, scratching it: the hand on the head,
  // then lifted a pixel.
  scratch: [layer(['###.', '...#', '...#', '...#', '..#.'], 15, 4)],
  scratchUp: [layer(['#...', '.##.', '...#', '...#', '...#', '..#.'], 15, 3)],
  up: [layer(['#.', '.#'], 5, 7), layer(['.#', '#.'], 17, 7)],
  out: [layer(['##'], 5, 9), layer(['##'], 17, 9)],
  // Typing: one arm forward, then the other.
  leftOut: [layer(['##'], 5, 9), layer(['#.', '.#'], 17, 10)],
  rightOut: [layer(['.#', '#.'], 5, 10), layer(['##'], 17, 9)],
}
type Arms = 'none' | 'scratch' | 'scratchUp' | 'up' | 'out' | 'leftOut' | 'rightOut'

const OPEN_MOUTH = layer(['oo', 'oo'], 11, 11)

// Above the head.
const BANG = layer(['##', '##', '..', '##'], 11, 0)
const QUESTION = layer(['.##.', '#..#', '..#.', '....', '..#.'], 10, 0)
const ZZ = ['###', '.#.', '###']
const BALL = ['##', '##']

type Pose = {
  // The whole drop moved by (dx, dy).
  dx?: number
  dy?: number
  eyes?: Eyes
  // Where the eyes look, in pixels from the middle.
  look?: [number, number]
  arms?: Arms
  mouth?: boolean
  // Drawn as they are, not moved with the drop.
  extra?: Layer[]
}

// The drop in a pose.
function pose(p: Pose): Frame {
  const { dx = 0, dy = 0, eyes = 'open', look = [0, 0], arms = 'none', extra = [] } = p
  const move = (l: Layer) => moved(l, dx, dy)
  const [lx, ly] = look
  return compose(
    WIDTH,
    HEIGHT,
    move(layer(BODY, 7, 5)),
    move(layer(EYES[eyes], 9 + lx, 8 + ly)),
    move(layer(EYES[eyes], 13 + lx, 8 + ly)),
    ...(p.mouth ? [move(OPEN_MOUTH)] : []),
    ...ARMS[arms].map(move),
    ...extra,
  )
}

// Another body shape at (x, y), its eyes at (ex, ey).
const shape = (body: string[], x: number, y: number, eyes: string[], ex: number, ey: number) =>
  compose(WIDTH, HEIGHT, layer(body, x, y), layer(eyes, ex, ey), layer(eyes, ex + 4, ey))

const rest = pose({})
const blink = (['half', 'closed', 'half'] as const).map((eyes) => pose({ eyes }))
const squash = shape(SQUASH, 6, 7, EYES.open, 9, 9)
const stretch = shape(STRETCH, 8, 3, EYES.open, 9, 6)

// --- Resting ---

const lookAround = [
  ...hold(pose({ look: [-1, 0] }), 6),
  ...hold(pose({ look: [1, 0] }), 6),
  ...hold(rest, 4),
]

// Melts into a puddle to sleep, "z"s rising, then takes shape again.
const melting = [
  rest,
  shape(SQUASH, 6, 7, EYES.half, 9, 9),
  shape(FLAT, 5, 9, EYES.half, 9, 11),
  compose(WIDTH, HEIGHT, layer(PUDDLE, 4, 11)),
]
const puddle = (z: Layer[]) => compose(WIDTH, HEIGHT, layer(PUDDLE, 4, 11), ...z)
const nap = [
  ...hold(puddle([]), 4),
  ...hold(puddle([layer(ZZ, 16, 6)]), 4),
  ...hold(puddle([layer(ZZ, 17, 3)]), 4),
  ...hold(puddle([layer(ZZ, 18, 0)]), 4),
]

// --- Working ---

const bounce = [
  rest,
  pose({ dy: -1, arms: 'out' }),
  pose({ dy: -2, arms: 'up' }),
  pose({ dy: -2, arms: 'up' }),
  pose({ dy: -1, arms: 'out' }),
  rest,
  squash,
  squash,
]
const typing = times(
  [pose({ look: [0, 1], arms: 'leftOut' }), pose({ look: [0, 1], arms: 'rightOut' })],
  6,
)

// Rolls: the eyes go round the body while it moves sideways; rolling to
// the right turns clockwise (top, right, bottom, left).
const AROUND: [number, number][] = [
  [0, -1],
  [2, 1],
  [0, 3],
  [-2, 1],
]
const rolling = (from: number, to: number) => {
  const step = to > from ? 1 : -1
  return Array.from({ length: Math.abs(to - from) + 1 }, (_, i) => {
    const turn = ((step * i) % 4 + 4) % 4
    return pose({ dx: from + step * i, look: AROUND[turn] })
  })
}
const roll = [
  ...rolling(0, 5),
  ...hold(pose({ dx: 5, look: [-1, 0] }), 4),
  ...rolling(5, -5),
  ...hold(pose({ dx: -5, look: [1, 0] }), 4),
  ...rolling(-5, 0),
  ...hold(rest, 2),
]

// Wobbles like jelly, then settles.
const wobble = [...times([stretch, rest, squash, rest], 2), stretch, rest, rest]

// Plays with a ball: it bounces on its head, flies off to the right and
// comes back.
const ballAt = (x: number, y: number, p: Pose = {}) =>
  pose({ look: [0, -1], ...p, extra: [layer(BALL, x, y)] })
const header = [
  ballAt(11, 3, { dy: 1 }),
  ballAt(11, 1),
  ballAt(11, 0),
  ballAt(11, 0),
  ballAt(11, 1),
]
const ball = [
  ...times(header, 3),
  ballAt(11, 3, { dy: 1 }),
  ballAt(14, 1, { look: [1, -1] }),
  ballAt(17, 0, { look: [1, -1] }),
  ballAt(20, 1, { look: [1, 0] }),
  ballAt(22, 3, { look: [1, 0] }),
  ...hold(pose({ look: [1, 0] }), 4),
  ballAt(22, 3, { look: [1, 0] }),
  ballAt(20, 1, { look: [1, -1] }),
  ballAt(17, 0, { look: [1, -1] }),
  ballAt(14, 1),
  ...times(header, 2),
  ...hold(rest, 2),
]

// --- Asking ---

// Mouth open, under a "!" or not.
const calling = (bang: boolean) => pose({ mouth: true, extra: bang ? [BANG] : [] })
const shaking = (dx: number) => pose({ dx, mouth: true })

// Lowered by one pixel, so the question mark fits above; scratching its
// head (`up`: the hand lifted, between two scratches).
const wondering = (look: [number, number], eyes: Eyes = 'open', up = false) =>
  pose({ dy: 1, look, eyes, arms: up ? 'scratchUp' : 'scratch', extra: [QUESTION] })
// `count` pictures of wondering, the hand going up and down every two.
const scratching = (look: [number, number], count: number) =>
  Array.from({ length: count }, (_, i) => wondering(look, 'open', Math.floor(i / 2) % 2 === 1))

// --- Done and failed ---

const xEyes = ['o.o', '.o.', 'o.o']
const plusEyes = ['.o.', 'ooo', '.o.']
const flat = (eyes: string[]) =>
  compose(WIDTH, HEIGHT, layer(FLAT, 5, 9), layer(eyes, 8, 10), layer(eyes, 13, 10))

// --- Moods (the Companion module, DF-0024) ---

const NOTE = ['.##', '.#.', '##.']
const note = (x: number, y: number) => layer(NOTE, x, y)
// Speed lines behind a runner.
const lines = (y: number) => [layer(['###', '...', '.##'], 0, y)]
const HEART = ['#.#', '###', '.#.']

// Dances: sways side to side, arms up, eyes half shut, notes around.
const dance = times(
  [
    ...hold(pose({ dx: -1, arms: 'up', eyes: 'half', extra: [note(2, 4)] }), 2),
    ...hold(pose({ dy: -1, arms: 'out', eyes: 'half', extra: [note(2, 2)] }), 2),
    ...hold(pose({ dx: 1, arms: 'up', eyes: 'half', extra: [note(19, 4)] }), 2),
    ...hold(pose({ dy: -1, arms: 'out', eyes: 'half', extra: [note(19, 2)] }), 2),
  ],
  2,
)
// Runs on the spot: bobbing, arms pumping.
const run = times(
  [
    pose({ look: [1, 0], arms: 'leftOut', extra: lines(8) }),
    pose({ dy: -1, look: [1, 0], arms: 'rightOut', extra: lines(9) }),
  ],
  4,
)
// Awake, looking at you.
const watch = [...hold(rest, 14), ...blink, ...hold(pose({ look: [0, -1] }), 6)]
// Petted: blissful, hearts rising.
const petted = Array.from({ length: 8 }, (_, step) =>
  pose({
    eyes: 'half',
    extra: [layer(HEART, 3, 6 - step), ...(step >= 2 ? [layer(HEART, 19, 8 - step)] : [])],
  }),
)
// Stretches tall, eyes shut, then blinks.
const stretching = [
  rest,
  ...hold(shape(STRETCH, 8, 3, EYES.closed, 9, 6), 6),
  squash,
  rest,
  ...blink,
]

export const bloop: Companion = {
  width: WIDTH,
  height: HEIGHT,
  animations: {
    // Waiting for a prompt: blinks, looks around once, melts into a puddle,
    // then naps there ("z"s) as long as it waits.
    neutral: {
      fps: 8,
      intro: [...hold(rest, 6), ...blink, ...lookAround, ...melting],
      frames: nap,
    },
    // Working: bounces, types, rolls about, wobbles, plays with a ball.
    active: {
      fps: 8,
      frames: [
        ...times(bounce, 2),
        ...typing,
        ...roll,
        ...typing,
        ...wobble,
        ...ball,
        ...typing,
      ],
    },
    // A permission to give: mouth open under a blinking "!", then shakes.
    attention: {
      fps: 8,
      frames: [
        ...times([...hold(calling(true), 3), ...hold(calling(false), 2)], 3),
        ...times([shaking(-1), shaking(1)], 4),
        ...hold(pose({ mouth: true }), 4),
      ],
    },
    // Waiting for an answer: scratches its head under a "?", looks up,
    // left, right.
    question: {
      fps: 8,
      frames: [
        ...scratching([0, -1], 8),
        ...scratching([-1, 0], 6),
        ...scratching([1, 0], 6),
        ...scratching([0, 0], 6),
        ...(['half', 'closed', 'half'] as const).map((eyes) => wondering([0, 0], eyes)),
        ...hold(wondering([0, 0]), 4),
      ],
    },
    // Done: confetti falls once, then it stays still (a moving success
    // distracts, DF-0020); the color says it.
    success: {
      fps: 8,
      intro: confetti(WIDTH, 14, 12).map((pieces) => pose({ extra: pieces })),
      frames: [rest],
    },
    // Failed: falls flat, then lies there with crossed eyes, dazed now and
    // then.
    error: {
      fps: 8,
      intro: [rest, squash, flat(xEyes)],
      frames: [...hold(flat(xEyes), 12), ...hold(flat(plusEyes), 2)],
    },
  },
  moods: {
    // A quiet day: its scenes in a random order (rolling about, wobbling, a
    // ball, a nap in a puddle…), a calm pause before each.
    idle: {
      fps: 8,
      frames: [...hold(rest, 20), ...blink, ...hold(rest, 8)],
      scenes: [
        lookAround,
        roll,
        [...melting, ...times(nap, 3), ...[...melting].reverse(), ...blink],
        wobble,
        ball,
      ],
    },
    dance: { fps: 8, frames: dance },
    run: { fps: 8, frames: run },
    watch: { fps: 8, frames: watch },
    petted: { fps: 8, intro: petted, frames: [pose({ eyes: 'half' })] },
    stretch: { fps: 8, intro: stretching, frames: [rest] },
  },
}
