import type { SoundKind } from './sound'

// Mirrors `Tone` in src-tauri/src/module.rs: colour, urgency and sound of an item.
export type Tone = 'neutral' | 'active' | 'attention' | 'question' | 'success' | 'error'

// Mirrors `ActionIcon` in src-tauri/src/module.rs
export type ActionIcon = 'play' | 'pause' | 'reset'

// Mirrors `Action` in src-tauri/src/module.rs: a button at the end of a row.
export interface Action {
  id: string
  // Tooltip of an icon button, text of a button without icon ("+1 min").
  label: string
  icon: ActionIcon | null
}

// Mirrors `Item` in src-tauri/src/module.rs: one line of the notch.
export interface Item {
  // `<module id>:<item id>`
  id: string
  title: string
  label: string
  detail: string | null
  tone: Tone
  // Shows a dot in the tone's colour; text-only items (the time) have none.
  dot: boolean
  actions: Action[]
}

// Mirrors `Section` in src-tauri/src/module.rs: one module's part of the notch.
export interface Section {
  // Module id, picks the section's icon.
  module: string
  items: Item[]
  // The module's placeholder when it has no item, shown in the open notch.
  note: string | null
}

// Mirrors `Content` in src-tauri/src/module.rs: sections in the user's order.
export interface Content {
  sections: Section[]
  // Shown in the open notch when no module is enabled.
  note: string | null
}

export function allItems(content: Content): Item[] {
  return content.sections.flatMap((s) => s.items)
}

// Screen edge the notch is attached to, mirrors `Edge` in src-tauri/src/placement.rs
export type Edge = 'top' | 'bottom' | 'left' | 'right'

// Notch glued to the edge or pill detached from it, mirrors `Style` in
// src-tauri/src/placement.rs
export type Style = 'notch' | 'pill'

// Modules all in the notch, or one card each, mirrors `ModuleLayout` in
// src-tauri/src/config.rs (DF-0011)
export type ModuleLayout = 'joined' | 'separate'

// A drawn card: the notch first, then, in the separate layout, one card per
// other module. Sizes in logical pixels, mirrored by `notch::Card` in Rust.
export interface Card {
  width: number
  height: number
  radius: number
  // Modules shown in it when the notch is open.
  sections: Section[]
}

// Mirrors `NotchSpeed` in src-tauri/src/config.rs
export type NotchSpeed = 'slow' | 'normal' | 'fast'

// Length of the opening / closing animation, in ms.
export const speedMs: Record<NotchSpeed, number> = { slow: 400, normal: 250, fast: 150 }

// Mirrors `Status` in src-tauri/src/lib.rs
export interface Status {
  soundEnabled: boolean
  edge: Edge
  style: Style
  // Move mode: the notch can be dragged along its edge (DF-0006)
  movable: boolean
  layout: ModuleLayout
  speed: NotchSpeed
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

// Module of an item, from its `<module id>:<item id>`.
export function moduleOf(item: Item): string {
  return item.id.split(':')[0]
}

// What the compact notch shows (DF-0011): the module with the most urgent
// alert, else the first module with items, in the user's order.
export function compactSection(sections: Section[]): Section | undefined {
  let alert: Section | undefined
  let rank = Infinity
  for (const section of sections) {
    for (const item of section.items) {
      const r = priority.indexOf(item.tone)
      if (soundFor(item.tone) && r < rank) {
        alert = section
        rank = r
      }
    }
  }
  return alert ?? sections.find((s) => s.items.length > 0)
}
