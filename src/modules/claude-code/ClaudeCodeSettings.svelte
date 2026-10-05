<script lang="ts">
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
      {data.hooksInstalled ? 'Connecté' : 'Non connecté'}
    </div>
    {#if data.hooksPath}<div class="hint path">Hooks dans {data.hooksPath}</div>{/if}
  </div>
  <button
    class:primary={!data.hooksInstalled}
    onclick={() => call('connect', { connected: !data.hooksInstalled })}
  >
    {data.hooksInstalled ? 'Déconnecter' : 'Connecter'}
  </button>
</div>
{#if data.serverError}
  <p class="error">{data.serverError} : change le port ci-dessous.</p>
{/if}
<form
  class="row separated"
  onsubmit={(e) => {
    e.preventDefault()
    void call('set_port', { port: Number(port) })
  }}
>
  <div>
    <div class="label">Port du serveur local</div>
    <div class="hint">Claude Code est reconnecté automatiquement.</div>
  </div>
  <div class="field">
    <input type="number" min="1024" max="65535" bind:value={port} />
    <button disabled={port === String(data.serverPort)}>Appliquer</button>
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
    <div class="label">Oublier une session inactive après</div>
    <div class="hint">En minutes, si son terminal a été fermé brutalement.</div>
  </div>
  <div class="field">
    <input type="number" min="5" max="1440" bind:value={timeout} />
    <button disabled={timeout === String(data.sessionTimeoutMinutes)}>Appliquer</button>
  </div>
</form>
