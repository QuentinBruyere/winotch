<script lang="ts">
  import { t, type Key } from '../../lib/i18n.svelte'
  import type { ModuleSettingsProps } from '..'

  // Mirrors `ClaudeCode::settings` in src-tauri/src/modules/claude_code/mod.rs
  interface Data {
    hooksInstalled: boolean
    hooksPath: string | null
    serverPort: number
    serverError: string | null
    sessionTimeoutMinutes: number
  }

  let { data: raw, call }: ModuleSettingsProps = $props()
  const data = $derived(raw as Data)

  // Text fields are edited locally and applied on demand.
  let port = $state('')
  let timeout = $state('')
  $effect(() => {
    port = String(data.serverPort)
    timeout = String(data.sessionTimeoutMinutes)
  })
</script>

<div class="row separated">
  <div>
    <div class="label">
      <span class="status" class:on={data.hooksInstalled}></span>
      {data.hooksInstalled ? t('claude-code.state_connected') : t('claude-code.state_not_connected')}
    </div>
    {#if data.hooksPath}<div class="hint path">{t('claude-code.hooks_in', { path: data.hooksPath })}</div>{/if}
  </div>
  <button
    class:primary={!data.hooksInstalled}
    onclick={() => call('connect', { connected: !data.hooksInstalled })}
  >
    {data.hooksInstalled ? t('claude-code.disconnect') : t('claude-code.connect')}
  </button>
</div>
{#if data.serverError}
  <p class="error">{t('claude-code.server_error', { error: data.serverError })}</p>
{/if}
<form
  class="row separated"
  onsubmit={(e) => {
    e.preventDefault()
    void call('set_port', { port: Number(port) })
  }}
>
  <div>
    <div class="label">{t('claude-code.port')}</div>
    <div class="hint">{t('claude-code.port_hint')}</div>
  </div>
  <div class="field">
    <input type="number" min="1024" max="65535" bind:value={port} />
    <button disabled={port === String(data.serverPort)}>{t('settings.apply')}</button>
  </div>
</form>
<form
  class="row separated"
  onsubmit={(e) => {
    e.preventDefault()
    void call('set_session_timeout', { minutes: Number(timeout) })
  }}
>
  <div>
    <div class="label">{t('claude-code.timeout')}</div>
    <div class="hint">{t('claude-code.timeout_hint')}</div>
  </div>
  <div class="field">
    <input type="number" min="5" max="1440" bind:value={timeout} />
    <button disabled={timeout === String(data.sessionTimeoutMinutes)}>{t('settings.apply')}</button>
  </div>
</form>
