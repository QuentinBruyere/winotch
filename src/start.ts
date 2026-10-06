import { getCurrentWindow } from '@tauri-apps/api/window'
import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import type { ModuleUis } from './modules'
import Settings from './settings/Settings.svelte'

// One front-end, two windows: the notch and the settings. A build that adds
// modules passes their icons and settings pages too (ADR-0009).
export function start(moduleUis: ModuleUis) {
  const label = getCurrentWindow().label
  document.documentElement.dataset.window = label
  const target = document.getElementById('app')!
  return label === 'settings'
    ? mount(Settings, { target, props: { moduleUis } })
    : mount(App, { target, props: { moduleUis } })
}
