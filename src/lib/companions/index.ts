import type { CompanionSize } from '../content'
import { bit } from './bit'
import { bloop } from './bloop'
import { miso } from './miso'
import { OVERLAP, RATIO, type Companion } from './companion'

// The companions shipped with winotch, by id: `crate::companion::BUILT_IN`
// on the Rust side lists the same ids.
export const companions: Record<string, Companion> = {
  bloop,
  bit,
  miso,
}

// A companion's shown size in CSS pixels, in an open notch's row (24 px
// high) and in the closed notch or a pin (36 px thick). Small draws the
// 16 × 16 pictures pixel for pixel.
export const SIZES: Record<CompanionSize, { row: number; compact: number }> = {
  small: { row: 16, compact: 16 },
  medium: { row: 20, compact: 24 },
  large: { row: 24, compact: 32 },
}

// The width a companion takes in an open notch's row: a dot in the same
// module is given as much, so the titles line up.
export const rowWidth = (size: CompanionSize) => SIZES[size].row * (RATIO - 2 * OVERLAP)
