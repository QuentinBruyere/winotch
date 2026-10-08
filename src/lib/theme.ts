// Mirrors `Appearance` in src-tauri/src/config.rs (DF-0014).
export type Appearance = 'system' | 'light' | 'dark'

const systemDark = window.matchMedia('(prefers-color-scheme: dark)')
let chosen: Appearance = 'system'
// The notch's own color (DF-0019), `#rrggbb`; null = the theme's.
let notchColor: string | null = null
// How opaque its background is, 0 to 100, and how much grain on it, 0 to
// `MAX_GRAIN`.
let notchOpacity = 100
let notchGrain = 0

// Sets `data-theme` (the page: `light` or `dark`) and `data-notch` (the
// notch's texts and buttons: dark ones on a light notch, light ones on a
// dark one) on the page: the stylesheets pick their colors from them.
// `system` follows the Windows app mode, live.
function apply() {
  const root = document.documentElement
  const dark = chosen === 'system' ? systemDark.matches : chosen === 'dark'
  root.dataset.theme = dark ? 'dark' : 'light'
  if (notchColor) {
    root.dataset.notch = needsDarkText(notchColor) ? 'light' : 'dark'
    root.style.setProperty('--notch-bg', notchColor)
  } else {
    root.dataset.notch = root.dataset.theme
    root.style.removeProperty('--notch-bg')
  }
  root.style.setProperty('--notch-opacity', `${notchOpacity}%`)
  root.style.setProperty(
    '--notch-grain',
    `${(Math.min(notchGrain, MAX_GRAIN) / 100) * GRAIN_SCALE}`,
  )
}

export function applyAppearance(appearance: Appearance) {
  chosen = appearance
  apply()
}

export function setNotchColor(color: string | null, opacity: number, grain: number) {
  notchColor = color
  notchOpacity = opacity
  notchGrain = grain
  apply()
}

// The strongest grain, in % (`MAX_GRAIN` in src-tauri/src/config.rs): past
// it, it hides the notch's color and texts.
export const MAX_GRAIN = 20
// The grain layer's opacity for a 100 % grain (the layer's strength per %).
const GRAIN_SCALE = 0.5

// The texts of a light or dark notch (src/app.css), as relative luminances.
const DARK_TEXT = luminance('#1c1c1e')
const LIGHT_TEXT = luminance('#f2f2f2')

// Whether dark texts contrast more than light ones on `color` (WCAG ratio).
export function needsDarkText(color: string): boolean {
  const bg = luminance(color)
  const ratio = (a: number, b: number) => (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)
  return ratio(bg, DARK_TEXT) > ratio(bg, LIGHT_TEXT)
}

// WCAG relative luminance of `#rrggbb`.
function luminance(color: string): number {
  const channel = (i: number) => {
    const c = parseInt(color.slice(i, i + 2), 16) / 255
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

systemDark.addEventListener('change', apply)
