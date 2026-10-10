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

// Molf (DF-0020): Minim Notch's mascot, a wolf's head drawn as an outline,
// as on the logo, 24 × 16 pixels, in the middle. Its eyes are single
// pixels inside the outline; two little paws show under its head only
// while it types. Each picture is put together from the parts
// below.
//
// What it does, by state (★ = Molf's own, the others shared with every
// companion so far):
//
// | State     | Arrival (once)              | Loop                                        |
// |-----------|-----------------------------|---------------------------------------------|
// | neutral   | blinks, looks around, ★ a   | sleeps, head low ("z"s rising)              |
// |           | big yawn, eyes closing      |                                             |
// | active    | -                           | ★ types with its paws, ★ sniffs along the   |
// |           |                             | ground, ★ ears prick up while it scans      |
// | attention | -                           | ★ howls under a blinking "!", shakes        |
// | question  | -                           | looks up, left, right under a "?"           |
// | success   | two hearts float up         | still, ★ eyes closed, content               |
// | error     | ★ ears flat, growls         | ★ ears flat, eyes narrowed, an ear flicks   |
//
// Moods (the Companion module, DF-0024): awake and calm by day (idle,
// ★ howls at the sky now and then, sniffs), dances with notes, ★ runs seen
// from far (the whole wolf), watches you, petted (hearts), stretches with a yawn; alarm =
// attention, sleep = neutral.

const WIDTH = 24
const HEIGHT = 16

// The head at (6, 4), as on the logo: rows 4 to 13, columns 6 to 17. Ears
// on top, a V tuft between them, the muzzle at the bottom, open below.
const HEAD = [
  '..##....##..',
  '.##.#..#.##.',
  '.#..####..#.',
  '##...##...##',
  '#..........#',
  '#..........#',
  '#..........#',
  '#..........#',
  '#.#.####.#.#',
  '#.#.#..#.#.#',
]
// Ears laid back (angry): their tips folded out to the sides.
const HEAD_FLAT = [
  '............',
  '##........##',
  '.###.##.###.',
  '##...##...##',
  ...HEAD.slice(4),
]
// The left ear folded down for an instant (a flick).
const HEAD_FLICK = ['........##..', '.###...#.##.', ...HEAD.slice(2)]
// Ears pricked up, listening: one pixel taller.
const HEAD_PRICK = ['..#......#..', ...HEAD.slice(0, 1), ...HEAD.slice(1)]
const HEAD_X = 6
const HEAD_Y = 4

// Eyes: the left one at (8, 10), the right one at (15, 10), as on the logo.
const EYES = {
  open: ['#'],
  // Closed or narrowed: a little line, inwards.
  closed: ['##'],
  none: [],
}
type Eyes = keyof typeof EYES

// Its mouth open under the muzzle (howling, yawning, growling).
const MOUTH = layer(['#..#', '.##.'], 10, 14)
// Paws under the head: both, or one pressing down (typing, running).
const PAWS: Record<string, Layer[]> = {
  none: [],
  both: [layer(['##', '##'], 7, 14), layer(['##', '##'], 15, 14)],
  left: [layer(['##'], 7, 15), layer(['##'], 15, 14)],
  right: [layer(['##'], 7, 14), layer(['##'], 15, 15)],
}
type Paws = 'none' | 'both' | 'left' | 'right'

// Above and beside the head.
const BANG = layer(['#', '#', '.', '#'], 22, 2)
const QUESTION = layer(['.##.', '#..#', '..#.', '....', '..#.'], 19, 0)
const ZZ = ['###', '.#.', '###']
const HEART = ['#.#', '###', '.#.']
const NOTE = ['.##', '.#.', '##.']
const KEYBOARD = layer(['#.#.#.#.#.#.#.#'], 5, 15)
// A howl: little waves rising from its muzzle.
const WAVES = ['.#.#', '#.#.']
// Sniffing: a dot near the nose.
const SNIFF = ['#']

type Pose = {
  // The whole head moved by (dx, dy).
  dx?: number
  dy?: number
  head?: string[]
  eyes?: Eyes
  // Where the eyes look, in pixels from the middle.
  look?: [number, number]
  mouth?: boolean
  paws?: Paws
  // Drawn as they are, not moved with the head.
  extra?: Layer[]
}

// The wolf in a pose.
function pose(p: Pose): Frame {
  const { dx = 0, dy = 0, eyes = 'open', look = [0, 0], paws = 'none' } = p
  const move = (l: Layer) => moved(l, dx, dy)
  const head = p.head ?? HEAD
  // A taller head (ears pricked) starts one row higher.
  const top = HEAD_Y - (head.length - HEAD.length)
  const [lx, ly] = look
  // A closed eye's line goes inwards, so both stay inside the outline.
  const left = 8
  const right = eyes === 'closed' ? 14 : 15
  return compose(
    WIDTH,
    HEIGHT,
    move(layer(head, HEAD_X, top)),
    move(layer(EYES[eyes], left + lx, 10 + ly)),
    move(layer(EYES[eyes], right + lx, 10 + ly)),
    ...(p.mouth ? [move(MOUTH)] : []),
    ...PAWS[paws],
    ...(p.extra ?? []),
  )
}

const rest = pose({})
const blink = [pose({ eyes: 'none' }), rest]
const lookAround = [
  ...hold(pose({ look: [-1, 0] }), 4),
  ...hold(pose({ look: [1, 0] }), 4),
  ...hold(rest, 2),
]
// A big yawn, eyes shut, the head rising a little.
const yawn = [
  pose({ eyes: 'closed' }),
  ...hold(pose({ dy: -1, eyes: 'closed', mouth: true }), 5),
  pose({ eyes: 'closed' }),
]
// Asleep, head low, eyes shut, "z"s rising.
const sleeping = (z: Layer[]) => pose({ dy: 1, eyes: 'closed', extra: z })
const sleep = [
  ...hold(sleeping([layer(ZZ, 19, 4)]), 4),
  ...hold(sleeping([layer(ZZ, 20, 1)]), 4),
  ...hold(sleeping([]), 4),
]

// --- Working ---

// Types on a keyboard, one paw then the other. Its eyes stay level: one
// pixel lower they would touch its cheeks.
const typing = times(
  [
    pose({ paws: 'left', extra: [KEYBOARD] }),
    pose({ paws: 'right', extra: [KEYBOARD] }),
  ],
  8,
)
// Sniffs along the ground, nose down, from one side to the other.
const sniff = (dx: number, on: boolean) =>
  pose({ dx, dy: 1, extra: on ? [layer(SNIFF, 11 + dx, 15)] : [] })
const sniffing = [
  ...[-2, -2, -1, -1, 0, 0, 1, 1, 2, 2].map((dx, i) => sniff(dx, i % 2 === 0)),
  ...hold(pose({ dx: 2, look: [1, 0] }), 3),
  pose({ dx: 1 }),
  ...hold(rest, 2),
]
// Scans around, ears pricked.
const scan = [
  ...hold(pose({ head: HEAD_PRICK, look: [-1, 0] }), 4),
  ...hold(pose({ head: HEAD_PRICK, look: [1, 0] }), 4),
  ...hold(pose({ head: HEAD_PRICK }), 3),
  ...blink,
]

// --- Asking ---

// Howls, head up, waves rising, under a "!" or not.
const howling = (bang: boolean, wave: number) =>
  pose({
    dy: -1,
    eyes: 'closed',
    mouth: true,
    extra: [layer(WAVES, 2, 6 - wave), ...(bang ? [BANG] : [])],
  })
const shaking = (dx: number) => pose({ dx, head: HEAD_PRICK })

// Looks around under a question mark.
const wondering = (look: [number, number], eyes: Eyes = 'open') =>
  pose({ look, eyes, head: HEAD_PRICK, extra: [QUESTION] })

// --- Done and failed ---

// Hearts floating up from beside its head.
const hearts = (step: number) => [
  layer(HEART, 2, 7 - step),
  ...(step >= 2 ? [layer(HEART, 19, 9 - step)] : []),
]
const growling = (dx: number) => pose({ dx, head: HEAD_FLAT, eyes: 'closed', mouth: true })
const grumpy = pose({ head: HEAD_FLAT, eyes: 'closed' })

// --- Moods (the Companion module, DF-0024) ---

const note = (x: number, y: number) => layer(NOTE, x, y)

// Dances: head bobbing side to side, ears flicking, notes around.
const dance = times(
  [
    ...hold(pose({ dx: -1, dy: -1, eyes: 'closed', extra: [note(1, 4)] }), 2),
    ...hold(pose({ head: HEAD_FLICK, eyes: 'closed', extra: [note(1, 2)] }), 2),
    ...hold(pose({ dx: 1, dy: -1, eyes: 'closed', extra: [note(20, 4)] }), 2),
    ...hold(pose({ eyes: 'closed', extra: [note(20, 2)] }), 2),
  ],
  2,
)
// Awake, looking at you, blinking now and then.
const watching = [...hold(rest, 10), ...blink, ...hold(rest, 6), ...lookAround]
// Petted: eyes shut, hearts rising.
const petted = Array.from({ length: 8 }, (_, step) => pose({ eyes: 'closed', extra: hearts(step) }))
// Stretches: head low, then a yawn, then a blink.
const stretching = [
  rest,
  ...hold(pose({ dy: 2, eyes: 'closed', paws: 'both' }), 3),
  ...yawn,
  rest,
  ...blink,
]
// A howl at the sky, then back to calm.
const skyHowl = [
  ...hold(pose({ head: HEAD_PRICK, look: [0, -1] }), 3),
  ...[0, 1, 2, 3, 0, 1, 2, 3].map((wave) => howling(false, wave)),
  ...hold(rest, 3),
]

// --- Seen from far away ---

// The whole wolf, tiny, side view, facing right (mirrored to face left):
// pointed ears, snout, a tail up behind. Its feet on row 14, the ground on
// row 15.
const TINY = {
  sit: ['.....#.', '#...###', '.#.##..', '.####..', '.#.##..'],
  // Running: legs stretched out, then gathered under.
  stretch: ['.......#.', '#.....###', '.#######.', '.#.....#.', '#.......#'],
  gather: ['.......#.', '.#....###', '..######.', '...#.#...', '..#...#..'],
  // Crouched, ready to pounce.
  crouch: ['.........', '.......#.', '#.....###', '.#######.', '.#.#.#.#.'],
}
type Tiny = keyof typeof TINY
const mirror = (rows: string[]) => rows.map((row) => [...row].reverse().join(''))
// The tiny wolf with its feet at `x` (its left side) and `lift` pixels up.
const tiny = (shape: Tiny, x: number, facing: 1 | -1, lift = 0): Layer =>
  layer(facing === 1 ? TINY[shape] : mirror(TINY[shape]), x, 10 - lift)
// Tufts of grass, far away.
const GRASS = '#.#..#...#.#...#..#.#..#'
const GROUND = layer([GRASS], 0, 15)
// A butterfly, wings open then closed; a fly, a single buzzing pixel.
const BUTTERFLY = [layer(['#.#', '.#.'], 0, 0), layer(['.#.', '.#.'], 0, 0)]
const butterfly = (x: number, y: number, step: number) => moved(BUTTERFLY[step % 2], x, y)
const far = (...layers: Layer[]) => compose(WIDTH, HEIGHT, GROUND, ...layers)
// Running feet: stretched and gathered in turn.
const stride = (step: number): Tiny => (step % 2 === 0 ? 'stretch' : 'gather')

// Runs, seen from far: the whole wolf in stride, bobbing, the grass going
// by under it (a running head alone made no sense).
const run = Array.from({ length: 8 }, (_, step) => {
  const shift = (step * 2) % GRASS.length
  const grass = layer([GRASS.slice(shift) + GRASS.slice(0, shift)], 0, 15)
  return compose(WIDTH, HEIGHT, grass, tiny(stride(step), 8, 1, step % 2))
})

// Chases a butterfly: it sits, sees it go by, runs after it, turns back
// when it does, and stops when it flies away up high.
const butterflyChase = (() => {
  const frames: Frame[] = []
  // The butterfly arrives from the left, over the sitting wolf.
  for (let s = 0; s < 8; s++) frames.push(far(tiny('sit', 9, 1), butterfly(1 + s, 3 + (s % 3), s)))
  // It goes on to the right; the wolf runs after it.
  for (let s = 0; s < 8; s++) {
    frames.push(far(tiny(stride(s), 8 + s, 1), butterfly(9 + s, 5 + (s % 2), s)))
  }
  // It turns back over the wolf; the wolf turns too and runs left.
  for (let s = 0; s < 12; s++) {
    frames.push(far(tiny(stride(s), 16 - s, -1), butterfly(17 - s, 4 - (s % 3), s)))
  }
  // It flies up and away; the wolf stops and watches it go.
  for (let s = 0; s < 6; s++) frames.push(far(tiny('sit', 4, -1), butterfly(5 - s, 2 - s, s)))
  frames.push(...hold(far(tiny('sit', 4, -1)), 6), ...hold(far(tiny('sit', 4, 1)), 4))
  return frames
})()

// Pounces on a fly buzzing over the grass: crouches, leaps, lands just
// short, and the fly buzzes off.
const flyPounce = (() => {
  const frames: Frame[] = []
  const fly = (x: number, y: number) => layer(['#'], x, y)
  // The fly buzzes around in front of it.
  const buzz: [number, number][] = [
    [16, 9], [17, 8], [16, 7], [18, 8], [17, 9], [16, 8], [18, 9], [17, 8],
  ]
  for (const [x, y] of buzz) frames.push(far(tiny('sit', 5, 1), fly(x, y)))
  // It crouches, eyes on it.
  for (const [x, y] of buzz.slice(0, 6)) frames.push(far(tiny('crouch', 5, 1), fly(x, y)))
  // The leap, in an arc.
  const arc = [2, 4, 5, 5, 4, 2, 0]
  arc.forEach((lift, s) =>
    frames.push(far(tiny('stretch', 6 + s, 1, lift), fly(17 + Math.floor(s / 2), 8 - s))),
  )
  // Missed: the fly is gone up; it sits and looks.
  frames.push(...hold(far(tiny('sit', 12, 1), fly(22, 1)), 2), ...hold(far(tiny('sit', 12, 1)), 8))
  return frames
})()

export const molf: Companion = {
  width: WIDTH,
  height: HEIGHT,
  animations: {
    // Waiting for a prompt: blinks, looks around, yawns, then sleeps head
    // low ("z"s) as long as it waits.
    neutral: {
      fps: 8,
      intro: [...hold(rest, 6), ...blink, ...lookAround, ...yawn, ...hold(sleeping([]), 2)],
      frames: sleep,
    },
    // Working: types, scans with its ears up, sniffs along the ground.
    active: {
      fps: 8,
      frames: [...typing, ...scan, ...typing, ...sniffing, ...typing, ...blink],
    },
    // A permission to give: howls under a blinking "!", then shakes.
    attention: {
      fps: 8,
      frames: [
        ...times([0, 1, 2, 3].map((wave) => howling(wave % 2 === 0, wave)), 3),
        ...times([shaking(-1), shaking(1)], 4),
        ...hold(pose({ head: HEAD_PRICK }), 4),
      ],
    },
    // Waiting for an answer: ears up under a "?", looks up, left, right.
    question: {
      fps: 8,
      frames: [
        ...hold(wondering([0, -1]), 8),
        ...hold(wondering([-1, 0]), 6),
        ...hold(wondering([1, 0]), 6),
        ...hold(wondering([0, 0]), 6),
        wondering([0, 0], 'none'),
        ...hold(wondering([0, 0]), 4),
      ],
    },
    // Done: two hearts float up once, then eyes closed, content, still (a
    // moving success distracts, DF-0020).
    success: {
      fps: 8,
      intro: Array.from({ length: 8 }, (_, step) => pose({ eyes: 'closed', extra: hearts(step) })),
      frames: [pose({ eyes: 'closed' })],
    },
    // Failed: ears flat, it growls once, then stays grumpy, an ear
    // flicking now and then.
    error: {
      fps: 8,
      intro: times([growling(-1), growling(1)], 3),
      frames: [...hold(grumpy, 12), pose({ head: HEAD_FLICK, eyes: 'closed' }), ...hold(grumpy, 2)],
    },
  },
  moods: {
    // A quiet day: its scenes in a random order (a howl at the sky,
    // sniffing, scanning, a nap…), a calm pause before each.
    idle: {
      fps: 8,
      frames: [...hold(rest, 20), ...blink, ...hold(rest, 6)],
      scenes: [
        butterflyChase,
        flyPounce,
        skyHowl,
        sniffing,
        scan,
        lookAround,
        [...yawn, ...times(sleep, 3), ...blink],
      ],
    },
    dance: { fps: 8, frames: dance },
    run: { fps: 8, frames: run },
    watch: { fps: 8, frames: watching },
    petted: { fps: 8, intro: petted, frames: [pose({ eyes: 'closed' })] },
    stretch: { fps: 8, intro: stretching, frames: [rest] },
  },
}
