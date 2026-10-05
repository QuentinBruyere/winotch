import { getCurrentWindow } from '@tauri-apps/api/window'
import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import type { SettingsComponents } from './modules'
import Settings from './settings/Settings.svelte'

// One front-end, two windows: the notch and the settings. A build that adds
// modules passes their settings components too (ADR-0009).
export function start(settingsComponents: SettingsComponents) {
  const label = getCurrentWindow().label
  document.documentElement.dataset.window = label
  const target = document.getElementById('app')!
  return label === 'settings'
    ? mount(Settings, { target, props: { settingsComponents } })
    : mount(App, { target })
}
