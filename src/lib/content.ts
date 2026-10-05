import type { SoundKind } from './sound'

// Mirrors `Tone` in src-tauri/src/module.rs: colour, urgency and sound of an item.
export type Tone = 'neutral' | 'active' | 'attention' | 'question' | 'success' | 'error'

// Mirrors `Item` in src-tauri/src/module.rs: one line of the notch.
export interface Item {
  // `<module id>:<item id>`
  id: string
  title: string
  label: string
  detail: string | null
  tone: Tone
}

// Mirrors `Content` in src-tauri/src/module.rs
export interface Content {
  items: Item[]
  // Messages of the enabled modules without items, shown in the open notch.
  notes: string[]
}

// Screen edge the notch is attached to, mirrors `Edge` in src-tauri/src/placement.rs
export type Edge = 'top' | 'bottom' | 'left' | 'right'

// Mirrors `Status` in src-tauri/src/lib.rs
export interface Status {
  soundEnabled: boolean
  edge: Edge
  // Move mode: the notch can be dragged along its edge (DF-0006)
  movable: boolean
  // Center of the compact notch along the edge, from the start of the window,
  // in logical pixels; null = the middle of the window (DF-0006)
  anchor: number | null
}

// Most urgent first: decides what the compact notch shows.
const priority: Tone[] = ['attention', 'error', 'question', 'success', 'active', 'neutral']

export function mostUrgent(items: Item[]): Item | undefined {
  return [...items].sort((a, b) => priority.indexOf(a.tone) - priority.indexOf(b.tone))[0]
}

export function toneColor(tone: Tone): string {
  return `var(--tone-${tone})`
}

// Tones that deserve attention: they expand the notch and play a sound (DF-0003).
export function soundFor(tone: Tone): SoundKind | undefined {
  switch (tone) {
    case 'attention':
    case 'question':
      return 'attention'
    case 'success':
      return 'done'
    case 'error':
      return 'error'
    default:
      return undefined
  }
}

// Items that just entered an attention tone, compared to the previous list.
export function newAlerts(previous: Item[], next: Item[]): Item[] {
  const before = new Map(previous.map((i) => [i.id, i.tone]))
  return next.filter((i) => soundFor(i.tone) && before.get(i.id) !== i.tone)
}
