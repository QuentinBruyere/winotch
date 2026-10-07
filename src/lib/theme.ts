// Mirrors `Appearance` in src-tauri/src/config.rs (DF-0014).
export type Appearance = 'system' | 'light' | 'dark'

const systemDark = window.matchMedia('(prefers-color-scheme: dark)')
let chosen: Appearance = 'system'

// Sets `data-theme` to `light` or `dark` on the page: the stylesheets pick
// their colors from it. `system` follows the Windows app mode, live.
export function applyAppearance(appearance: Appearance) {
  chosen = appearance
  const dark = appearance === 'system' ? systemDark.matches : appearance === 'dark'
  document.documentElement.dataset.theme = dark ? 'dark' : 'light'
}

systemDark.addEventListener('change', () => applyAppearance(chosen))
