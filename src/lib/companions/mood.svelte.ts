import type { Section, Tone } from '../content'

// The mood of a companion that reacts to the notch (the Companion module,
// DF-0024), from what the notch shows and the time of day; the mouse
// (watch, petted) is added by the companion itself.
export type Mood =
  | 'alarm'
  | 'watch'
  | 'petted'
  | 'dance'
  | 'run'
  | 'stretch'
  | 'idle'
  | 'sleep'

// Between these hours it is awake (idle when nothing happens) and stretches
// at each o'clock; the rest of the time, it sleeps unless the notch wakes it.
const DAY_START = 7
const DAY_END = 23
// How long it stretches at an o'clock.
const STRETCH_MS = 4000

// What the notch shows, set by App.svelte.
let sections = $state<Section[]>([])
let stretching = $state(false)
let night = $state(isNight(new Date().getHours()))

function isNight(hour: number) {
  return hour < DAY_START || hour >= DAY_END
}

export function setNotchSections(list: Section[]) {
  sections = list
}

// Its mood from the notch: a ringing item first, then music, something
// running, the o'clock stretch, else awake by day, asleep by night.
export function notchMood(): Mood {
  const items = sections.flatMap((s) => s.items)
  if (items.some((i) => i.ringing)) return 'alarm'
  if (items.some((i) => i.activity === 'music')) return 'dance'
  if (items.some((i) => i.activity === 'running')) return 'run'
  if (stretching) return 'stretch'
  return night ? 'sleep' : 'idle'
}

// The tone whose color a mood wears: alert for an alarm, else always the
// active (or chosen) color, asleep too: it does not darken.
export function moodTone(mood: Mood): Tone {
  return mood === 'alarm' ? 'attention' : 'active'
}

// Wakes it for a stretch at each o'clock of the day.
function nextOclock() {
  const now = new Date()
  const next = new Date(now)
  next.setHours(now.getHours() + 1, 0, 0, 0)
  setTimeout(() => {
    const hour = new Date().getHours()
    night = isNight(hour)
    if (!night) {
      stretching = true
      setTimeout(() => (stretching = false), STRETCH_MS)
    }
    nextOclock()
  }, next.getTime() - now.getTime())
}

nextOclock()
