import type { SoundKind } from './sound'
import type { Appearance } from './theme'

// Mirrors `Tone` in src-tauri/src/module.rs: colour, urgency and sound of an item.
export type Tone = 'neutral' | 'active' | 'attention' | 'question' | 'success' | 'error'

// Mirrors `Icon` in src-tauri/src/module.rs
export type Icon =
  | 'play'
  | 'pause'
  | 'reset'
  | 'volume_low'
  | 'volume_medium'
  | 'volume_high'
  | 'muted'

// Mirrors `Action` in src-tauri/src/module.rs: a button at the end of a row.
export interface Action {
  id: string
  // Tooltip of an icon button, text of a button without icon ("+1 min").
  label: string
  icon: Icon | null
}

// Mirrors `Slider` in src-tauri/src/module.rs: moving it sends `set:<value>`.
export interface Slider {
  value: number
  min: number
  max: number
  step: number
}

// Where a slider's value sits, from 0 to 100.
export function sliderPercent(slider: Slider, value = slider.value): number {
  const span = slider.max - slider.min
  return span > 0 ? ((value - slider.min) / span) * 100 : 0
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
  // A slider in the open notch, between the title and the label.
  slider: Slider | null
  // Nothing worth showing yet: compact views show the module's icon instead.
  quiet: boolean
  // Replaces the module's icon in the compact views (the volume's level).
  icon: Icon | null
  // Rings (a timer whose time is up): an alarm repeats and its shape pulses.
  ringing: boolean
}

// Mirrors `Section` in src-tauri/src/module.rs: one module's part of the notch.
export interface Section {
  // Module id, picks the section's icon.
  module: string
  items: Item[]
  // The module's placeholder when it has no item, shown in the open notch.
  note: string | null
  // Pinned: shown in its own mini-notch on this side of the notch (DF-0012).
  pin: Side | null
}

// Side of a pin, mirrors `Side` in src-tauri/src/module.rs. On the left and
// right edges of the screen: above and below the notch.
export type Side = 'left' | 'right'

// A rectangle in logical pixels, relative to the window.
export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

// A visible shape for the window region, mirrors `notch::Shape` in Rust.
export interface Shape extends Rect {
  radius: number
  // Glued to the screen edge: square corners on that side.
  attached: boolean
}

// The box around two places of a moving shape: every place in between too.
// Its corners are the rounder one's, so neither shape pokes out of them.
export function around(a: Shape, b: Shape): Shape {
  const x = Math.min(a.x, b.x)
  const y = Math.min(a.y, b.y)
  return {
    x,
    y,
    width: Math.max(a.x + a.width, b.x + b.width) - x,
    height: Math.max(a.y + a.height, b.y + b.height) - y,
    radius: Math.min(a.radius, b.radius),
    attached: a.attached && b.attached,
  }
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

// Whether one of these modules rings: the shape showing them pulses.
export function ringing(sections: Section[]): boolean {
  return sections.some((s) => s.items.some((i) => i.ringing))
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
  appearance: Appearance
  // The language shown (ADR-0012), the system's already resolved.
  language: string
  // Center of the compact notch along the edge, from the start of the window,
  // in logical pixels; null = the middle of the window (DF-0006)
  anchor: number | null
}

// CSS corners of a shape: glued to the edge, a notch rounds its inner
// corners only (DF-0006); a pill or a detached card rounds all four.
export function corners(edge: Edge, radius: number, attached: boolean): string {
  const r = radius
  if (!attached) return `${r}px`
  return {
    top: `0 0 ${r}px ${r}px`,
    bottom: `${r}px ${r}px 0 0`,
    left: `0 ${r}px ${r}px 0`,
    right: `${r}px 0 0 ${r}px`,
  }[edge]
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
