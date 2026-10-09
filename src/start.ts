import { getCurrentWindow } from '@tauri-apps/api/window'
import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { watchCustomCompanions } from './lib/companions/custom.svelte'
import type { ModuleUis } from './modules'
import Settings from './settings/Settings.svelte'

// One front-end, two windows: the notch and the settings. A build that adds
// modules passes their icons and settings pages too (ADR-0009).
export function start(moduleUis: ModuleUis) {
  const label = getCurrentWindow().label
  document.documentElement.dataset.window = label
  // The notch is not a web page: no browser menu on right click.
  if (label !== 'settings') document.addEventListener('contextmenu', (e) => e.preventDefault())
  // Imported companions (DF-0025): drawn in the notch, listed in the settings.
  watchCustomCompanions()
  const target = document.getElementById('app')!
  return label === 'settings'
    ? mount(Settings, { target, props: { moduleUis } })
    : mount(App, { target, props: { moduleUis } })
}
