import type { Component } from 'svelte'
import ClaudeCodeSettings from './claude-code/ClaudeCodeSettings.svelte'

// What a module's settings component receives (ADR-0009).
export interface ModuleSettingsProps {
  // The module's own data, from `Module::settings` in Rust.
  data: unknown
  // Runs one of the module's actions (`Module::call`); errors are shown by the window.
  call: (action: string, args?: Record<string, unknown>) => Promise<void>
}

// Settings sections of the modules shipped with the core, by module id.
export const settingsComponents: Record<string, Component<ModuleSettingsProps>> = {
  'claude-code': ClaudeCodeSettings,
}
