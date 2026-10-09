import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { companions } from '.'
import type { Companion } from './companion'

// An imported companion (DF-0025), as `companion::Custom` on the Rust side:
// its pictures (made from its pack by the core), or why its pack is
// refused.
export type CustomCompanion = {
  id: string
  name: string
  definition: Companion | null
  error: string | null
}

// Raw: the pictures are big and never changed in place, only replaced.
let custom = $state.raw<CustomCompanion[]>([])
let watching = false

// Follows the imported companions, in every window (src/start.ts).
export function watchCustomCompanions() {
  if (watching) return
  watching = true
  void invoke<CustomCompanion[]>('get_companions').then((list) => (custom = list))
  void listen<CustomCompanion[]>('companions-changed', (e) => (custom = e.payload))
}

// The imported companions, refused packs included.
export function customCompanions(): CustomCompanion[] {
  return custom
}

// A companion by id: built in, or imported.
export function companionById(id: string): Companion | undefined {
  return companions[id] ?? custom.find((c) => c.id === id)?.definition ?? undefined
}
