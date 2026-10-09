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

// Miso (DF-0020): a little cat sitting face on, 24 × 16 pixels, in the
// middle; its tail on its right. A paw raised for a question; otherwise its
// paws stay down, except while working. Each picture is put together from
// the parts below.
//
// What it does, by state (★ = Miso's own, the others shared with every
// companion so far):
//
// | State     | Arrival (once)          | Loop                                          |
// |-----------|-------------------------|-----------------------------------------------|
// | neutral   | ★ slow cat blink,       | ★ sleeps curled into a loaf ("z"s rising)     |
// |           | ★ licks its paw, ★ curls|                                               |
// |           | into a loaf             |                                               |
// | active    | -                       | ★ taps a keyboard with its paws, ★ chases a   |
// |           |                         | ball of yarn, ★ ears twitch while it watches  |
// | attention | -                       | ears up, ★ tail puffed, under a blinking "!", |
// |           |                         | shakes                                        |
// | question  | -                       | raises a paw under a "?", looks up, left,     |
// |           |                         | right                                         |
// | success   | ★ two hearts float up   | still, ★ content squint                       |
// | error     | ★ fur bristles, hisses  | ★ ears flat, grumpy squint, tail flicks       |
//
// Moods (the Companion module, DF-0024): awake and calm by day (idle,
// ★ grooms), dances with notes (★ tail
// swinging), runs on the spot, watches you, petted (hearts), ★ a cat's
// stretch with a yawn; alarm = attention, sleep = neutral.

const WIDTH = 24
const HEIGHT = 16

// The head at (7, 3), ears included: rows 3 to 9, columns 7 to 16.
const HEAD = [
  '.#......#.',
  '.##....##.',
  '.########.',
  '##########',
  '##########',
  '##########',
  '.########.',
]
// Ears laid flat (cross): the head without its ear tips, ears to the sides.
const HEAD_FLAT = [
  '..........',
  '..........',
  '##########',
  '##########',
  '##########',
  '##########',
  '.########.',
]
// One ear twitching down (the left one).
const HEAD_TWITCH = [
  '........#.',
  '.......##.',
  '.########.',
  '##########',
  '##########',
  '##########',
  '.########.',
]
// The body under the head, at (7, 10): rows 10 to 13, its front paws at
// the bottom.
const BODY = ['..######..', '.########.', '.########.', '.###..###.']
// Fur bristling: the outline spiked.
const BRISTLES = layer(
  [
    '#.#........#.#',
    '..............',
    '#............#',
    '..............',
    '#............#',
    '..............',
    '#............#',
    '..............',
    '#............#',
    '..............',
    '.#.#......#.#.',
  ],
  5,
  3,
)

// Eyes: holes in the head, the left one at (x, y), the right one 4 pixels
// further.
const EYES = {
  open: ['oo', 'oo'],
  // A cat's slow blink, a content squint, or annoyed (with flat ears).
  half: ['..', 'oo'],
  closed: ['..', '..'],
}
type Eyes = keyof typeof EYES

const TAIL = {
  // Curled up beside the body.
  up: layer(['..#', '..#', '.#.', '#..'], 16, 9),
  mid: layer(['...#', '..#.', '##..'], 16, 10),
  down: layer(['....', '....', '####'], 16, 11),
  // Puffed up, alarmed.
  puffed: layer(['.##', '###', '.##', '##.', '#..'], 16, 8),
}
type Tail = keyof typeof TAIL

// Paws: none raised by default.
const PAWS: Record<string, Layer[]> = {
  none: [],
  // The right one raised, as to ask: up beside the head, gone from the
  // ground.
  raise: [layer(['.#', '.#', '#.'], 17, 5), layer(['ooo'], 13, 13)],
  // Licking: the left paw up at its mouth, gone from the ground.
  lick: [layer(['##', '.#'], 5, 9), layer(['ooo'], 8, 13)],
  // Tapping a keyboard: one paw down, then the other.
  leftDown: [layer(['##'], 9, 14)],
  rightDown: [layer(['##'], 13, 14)],
}
type Paws = 'none' | 'raise' | 'lick' | 'leftDown' | 'rightDown'

const OPEN_MOUTH = layer(['oo'], 11, 9)

// Above the head.
const BANG = layer(['##', '##', '..', '##'], 11, -1)
const QUESTION = layer(['.##.', '#..#', '..#.', '....', '..#.'], 18, 0)
const ZZ = ['###', '.#.', '###']
const HEART = ['#.#', '###', '.#.']
const KEYBOARD = layer(['#.#.#.#.#.#.#.#'], 5, 15)
const YARN = ['.#.', '###', '.#.']

type Pose = {
  // The whole cat moved by (dx, dy).
  dx?: number
  dy?: number
  head?: string[]
  eyes?: Eyes
  // Where the eyes look, in pixels from the middle.
  look?: [number, number]
  tail?: Tail
  paws?: Paws
  mouth?: boolean
  // Drawn as they are, not moved with the cat.
  extra?: Layer[]
}

// The cat in a pose.
function pose(p: Pose): Frame {
  const { dx = 0, dy = 0, eyes = 'open', look = [0, 0], tail = 'up', paws = 'none' } = p
  const move = (l: Layer) => moved(l, dx, dy)
  const [lx, ly] = look
  return compose(
    WIDTH,
    HEIGHT,
    move(layer(p.head ?? HEAD, 7, 3)),
    move(layer(BODY, 7, 10)),
    move(TAIL[tail]),
    move(layer(EYES[eyes], 9 + lx, 7 + ly)),
    move(layer(EYES[eyes], 13 + lx, 7 + ly)),
    ...(p.mouth ? [move(OPEN_MOUTH)] : []),
    ...PAWS[paws].map(move),
    ...(p.extra ?? []),
  )
}

const rest = pose({})
// A cat's slow blink: the eyes close slowly and stay shut a moment.
const slowBlink = [
  pose({ eyes: 'half' }),
  ...hold(pose({ eyes: 'closed' }), 4),
  pose({ eyes: 'half' }),
]

// --- Resting ---

const swish = [
  ...hold(pose({ tail: 'mid' }), 2),
  ...hold(pose({ tail: 'down' }), 2),
  ...hold(pose({ tail: 'mid' }), 2),
  ...hold(rest, 2),
]
// Licks its paw, eyes shut.
const groom = [
  ...times([pose({ eyes: 'closed', paws: 'lick' }), pose({ eyes: 'closed', paws: 'lick', dy: 1 })], 4),
  ...hold(rest, 2),
]
// Curled into a loaf, eyes shut.
const LOAF = [
  '.#........#.',
  '.##......##.',
  '############',
  '############',
  '############',
  '.##########.',
]
const loaf = (z: Layer[]) =>
  compose(
    WIDTH,
    HEIGHT,
    layer(LOAF, 6, 8),
    layer(['#####'], 17, 13),
    layer(['oo...oo'], 8, 11),
    ...z,
  )

// --- Working ---

// Taps a keyboard, one paw then the other, eyes down.
const typing = times(
  [
    pose({ look: [0, 1], paws: 'leftDown', extra: [KEYBOARD] }),
    pose({ look: [0, 1], paws: 'rightDown', extra: [KEYBOARD] }),
  ],
  8,
)
// Watches, ears twitching.
const watch = [
  ...hold(pose({ look: [1, 0] }), 3),
  pose({ look: [1, 0], head: HEAD_TWITCH }),
  ...hold(pose({ look: [1, 0] }), 3),
  pose({ look: [1, 0], head: HEAD_TWITCH }),
  ...hold(pose({ look: [-1, 0], tail: 'mid' }), 4),
]
// A ball of yarn rolls in from the right; the cat crouches, pounces, the
// ball rolls away and the cat comes back.
const yarn = (x: number) => [layer(YARN, x, 12)]
const chase = [
  ...[22, 21, 20, 19].map((x) => pose({ look: [1, 1], extra: yarn(x) })),
  ...hold(pose({ look: [1, 1], dy: 1, tail: 'down', extra: yarn(19) }), 3),
  pose({ dx: 2, dy: -1, look: [1, 1], tail: 'mid', extra: yarn(19) }),
  pose({ dx: 3, look: [1, 1], tail: 'mid', extra: yarn(20) }),
  ...[21, 22, 23].map((x) => pose({ dx: 3, look: [1, 0], extra: yarn(x) })),
  ...hold(pose({ dx: 3, look: [1, 0] }), 3),
  pose({ dx: 2 }),
  pose({ dx: 1 }),
  ...hold(rest, 2),
]

// --- Asking ---

// Ears up, tail puffed, under a "!" or not.
const calling = (bang: boolean) =>
  pose({ tail: 'puffed', mouth: true, extra: bang ? [BANG] : [] })
const shaking = (dx: number) => pose({ dx, tail: 'puffed', mouth: true })

// A paw raised, under a question mark.
const wondering = (look: [number, number], eyes: Eyes = 'open') =>
  pose({ look, eyes, paws: 'raise', extra: [QUESTION] })

// --- Done and failed ---

// Hearts floating up from beside its head.
const hearts = (step: number) => [
  layer(HEART, 2, 6 - step),
  ...(step >= 2 ? [layer(HEART, 19, 8 - step)] : []),
]

// --- Moods (the Companion module, DF-0024) ---

const NOTE = ['.##', '.#.', '##.']
const note = (x: number, y: number) => layer(NOTE, x, y)
// Speed lines behind a runner.
const lines = (y: number) => [layer(['###', '...', '.##'], 0, y)]

// Dances: head bobbing, tail swinging, eyes half shut, notes around.
const dance = times(
  [
    ...hold(pose({ dy: -1, tail: 'up', eyes: 'half', extra: [note(2, 4)] }), 2),
    ...hold(pose({ tail: 'mid', eyes: 'half', extra: [note(2, 2)] }), 2),
    ...hold(pose({ dy: -1, tail: 'down', eyes: 'half', extra: [note(20, 4)] }), 2),
    ...hold(pose({ tail: 'mid', eyes: 'half', extra: [note(20, 2)] }), 2),
  ],
  2,
)
// Runs on the spot: bobbing, paws swapping, tail streaming.
const run = times(
  [
    pose({ look: [1, 0], tail: 'down', paws: 'leftDown', extra: lines(9) }),
    pose({ dy: -1, look: [1, 0], tail: 'mid', paws: 'rightDown', extra: lines(10) }),
  ],
  4,
)
// Awake, looking at you, its tail swishing.
const watching = [
  ...hold(rest, 8),
  ...hold(pose({ tail: 'mid' }), 2),
  ...hold(pose({ tail: 'down' }), 2),
  ...hold(pose({ tail: 'mid' }), 2),
  ...slowBlink,
  ...hold(rest, 4),
]
// Petted: eyes shut, hearts rising.
const petted = Array.from({ length: 8 }, (_, step) => pose({ eyes: 'closed', extra: hearts(step) }))
// A cat's stretch: low in front, tail up, a yawn, then a slow blink.
const stretching = [
  rest,
  ...hold(pose({ dy: 2, tail: 'up', mouth: true, eyes: 'closed' }), 4),
  pose({ dy: 1, tail: 'up' }),
  rest,
  ...slowBlink,
]

export const miso: Companion = {
  width: WIDTH,
  height: HEIGHT,
  animations: {
    // Waiting for a prompt: a slow blink, licks its paw once, curls into a
    // loaf, then sleeps there ("z"s) as long as it waits.
    neutral: {
      fps: 8,
      intro: [
        ...hold(rest, 6),
        ...slowBlink,
        ...groom,
        pose({ eyes: 'half' }),
        ...hold(loaf([]), 2),
      ],
      frames: [
        ...hold(loaf([layer(ZZ, 18, 5)]), 4),
        ...hold(loaf([layer(ZZ, 19, 2)]), 4),
        ...hold(loaf([]), 4),
      ],
    },
    // Working: taps a keyboard, watches with twitching ears, chases a ball
    // of yarn.
    active: {
      fps: 8,
      frames: [...typing, ...watch, ...typing, ...chase, ...typing, ...swish],
    },
    // A permission to give: ears up, tail puffed, under a blinking "!",
    // then shakes.
    attention: {
      fps: 8,
      frames: [
        ...times([...hold(calling(true), 3), ...hold(calling(false), 2)], 3),
        ...times([shaking(-1), shaking(1)], 4),
        ...hold(pose({ tail: 'puffed' }), 4),
      ],
    },
    // Waiting for an answer: raises a paw under a "?", looks up, left, right.
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
    // Done: two hearts float up once, then a content squint, still (a
    // moving success distracts, DF-0020).
    success: {
      fps: 8,
      intro: Array.from({ length: 8 }, (_, step) =>
        pose({ eyes: 'half', extra: hearts(step) }),
      ),
      frames: [pose({ eyes: 'half' })],
    },
    // Failed: its fur bristles and it hisses once, then sits ears flat,
    // grumpy, its tail flicking now and then.
    error: {
      fps: 8,
      intro: [
        ...times(
          [
            pose({ head: HEAD_FLAT, tail: 'puffed', mouth: true, extra: [BRISTLES] }),
            pose({ head: HEAD_FLAT, tail: 'puffed', mouth: true }),
          ],
          3,
        ),
      ],
      frames: [
        ...hold(pose({ head: HEAD_FLAT, eyes: 'half', tail: 'down' }), 12),
        ...hold(pose({ head: HEAD_FLAT, eyes: 'half', tail: 'mid' }), 2),
      ],
    },
  },
  moods: {
    // A quiet day: its scenes in a random order (grooming, a ball of yarn,
    // watching, a nap as a loaf…), a calm pause before each.
    idle: {
      fps: 8,
      frames: [...hold(rest, 20), ...slowBlink, ...hold(rest, 6)],
      scenes: [
        swish,
        groom,
        chase,
        [
          pose({ eyes: 'half' }),
          ...times(
            [
              ...hold(loaf([]), 4),
              ...hold(loaf([layer(ZZ, 18, 5)]), 4),
              ...hold(loaf([layer(ZZ, 19, 2)]), 4),
            ],
            3,
          ),
          ...slowBlink,
        ],
        watch,
      ],
    },
    dance: { fps: 8, frames: dance },
    run: { fps: 8, frames: run },
    watch: { fps: 8, frames: watching },
    petted: { fps: 8, intro: petted, frames: [pose({ eyes: 'half' })] },
    stretch: { fps: 8, intro: stretching, frames: [rest] },
  },
}
