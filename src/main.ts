import { getCurrentWindow } from '@tauri-apps/api/window'
import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import Settings from './settings/Settings.svelte'

// One front-end, two windows: the notch and the settings.
const label = getCurrentWindow().label
document.documentElement.dataset.window = label

const app = mount(label === 'settings' ? Settings : App, {
  target: document.getElementById('app')!,
})

export default app
