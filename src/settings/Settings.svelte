<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'

  // Mirrors `Settings` in src-tauri/src/settings.rs
  type Strength = 'soft' | 'medium' | 'strong'
  type Edge = 'top' | 'bottom' | 'left' | 'right'
  interface Settings {
    soundEnabled: boolean
    hideInFullscreen: boolean
    edge: Edge
    screen: string | null
    screens: { id: string; label: string; primary: boolean }[]
    cursorResistance: boolean
    resistanceStrength: Strength
    resistanceAvailable: boolean
    serverPort: number
    sessionTimeoutMinutes: number
    autostart: boolean
    hooksInstalled: boolean
    serverError: string | null
    claudeSettingsPath: string
    configDir: string
  }

  const edgeChoices: { value: Edge; label: string }[] = [
    { value: 'top', label: 'Haut' },
    { value: 'bottom', label: 'Bas' },
    { value: 'left', label: 'Gauche' },
    { value: 'right', label: 'Droite' },
  ]

  type ResistanceChoice = 'off' | Strength
  const resistanceChoices: { value: ResistanceChoice; label: string }[] = [
    { value: 'off', label: 'Désactivée' },
    { value: 'soft', label: 'Douce' },
    { value: 'medium', label: 'Moyenne' },
    { value: 'strong', label: 'Forte' },
  ]

  let settings = $state<Settings | null>(null)
  let version = $state('')
  let error = $state<string | null>(null)
  // Text fields are edited locally and applied on demand.
  let port = $state('')
  let timeout = $state('')

  // The screen shown as selected: the chosen one, or the primary one by default.
  const selectedScreen = $derived(
    settings?.screen ?? settings?.screens.find((s) => s.primary)?.id ?? '',
  )
  const chosenUnplugged = $derived(
    !!settings?.screen && !settings.screens.some((s) => s.id === settings?.screen),
  )

  const resistance = $derived<ResistanceChoice>(
    settings?.cursorResistance ? settings.resistanceStrength : 'off',
  )

  function load(next: Settings) {
    settings = next
    port = String(next.serverPort)
    timeout = String(next.sessionTimeoutMinutes)
  }

  async function run(command: string, args: Record<string, unknown> = {}) {
    error = null
    try {
      await invoke(command, args)
    } catch (e) {
      error = String(e)
    }
  }

  function setResistance(choice: ResistanceChoice) {
    void run('set_cursor_resistance', {
      enabled: choice !== 'off',
      strength: choice === 'off' ? (settings?.resistanceStrength ?? 'medium') : choice,
    })
  }

  $effect(() => {
    const unlisten = listen<Settings>('settings-changed', (e) => load(e.payload))
    const refresh = () => invoke<Settings>('get_settings').then(load)
    void refresh()
    // A monitor may have been plugged in or out while the window was hidden.
    window.addEventListener('focus', refresh)
    getVersion().then((v) => (version = v))
    return () => {
      window.removeEventListener('focus', refresh)
      void unlisten.then((off) => off())
    }
  })
</script>

<main>
  <header>
    <h1>winotch</h1>
    {#if version}<span class="version">v{version}</span>{/if}
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if settings}
    <section>
      <h2>Claude Code</h2>
      <div class="row">
        <div>
          <div class="label">
            <span class="status" class:on={settings.hooksInstalled}></span>
            {settings.hooksInstalled ? 'Connecté' : 'Non connecté'}
          </div>
          <div class="hint">Hooks dans {settings.claudeSettingsPath}</div>
        </div>
        <button
          class:primary={!settings.hooksInstalled}
          onclick={() => run('set_claude_connected', { connected: !settings?.hooksInstalled })}
        >
          {settings.hooksInstalled ? 'Déconnecter' : 'Connecter'}
        </button>
      </div>
      {#if settings.serverError}
        <p class="error">{settings.serverError} : change le port dans la section Avancé.</p>
      {/if}
    </section>

    <section>
      <h2>Affichage</h2>
      <div class="stack">
        <div>
          <div class="label">Position du notch</div>
          <div class="hint">
            Centré sur le bord choisi. À gauche et à droite, le
            notch est fin et vertical : il s'ouvre au survol.
          </div>
        </div>
        <div class="segmented" role="radiogroup" aria-label="Position du notch">
          {#each edgeChoices as choice (choice.value)}
            <button
              role="radio"
              aria-checked={settings.edge === choice.value}
              class:selected={settings.edge === choice.value}
              onclick={() => run('set_edge', { edge: choice.value })}
            >
              {choice.label}
            </button>
          {/each}
        </div>
      </div>
      {#if settings.screens.length > 1 || chosenUnplugged}
        <label class="row separated">
          <div>
            <div class="label">Écran</div>
            <div class="hint">S'il est débranché, le notch revient sur l'écran principal.</div>
          </div>
          <select
            value={selectedScreen}
            onchange={(e) => run('set_screen', { screen: e.currentTarget.value })}
          >
            {#each settings.screens as screen (screen.id)}
              <option value={screen.id}>{screen.label}</option>
            {/each}
            {#if chosenUnplugged}
              <!-- Only while the chosen screen is unplugged: says why the
                   notch is on the primary screen for now. -->
              <option value={settings.screen}>Écran choisi (débranché)</option>
            {/if}
          </select>
        </label>
      {/if}
      <label class="row separated">
        <div>
          <div class="label">Masquer le notch en plein écran</div>
          <div class="hint">
            Jeux, vidéos, présentations. Le bureau et Alt+Tab ne comptent pas.
            {#if settings.resistanceAvailable}
              Désactivé, la résistance du curseur reste aussi active en plein écran.
            {/if}
          </div>
        </div>
        <input
          type="checkbox"
          class="switch"
          checked={settings.hideInFullscreen}
          onchange={(e) => run('set_hide_in_fullscreen', { enabled: e.currentTarget.checked })}
        />
      </label>
    </section>

    <section>
      <h2>Notifications</h2>
      <label class="row">
        <div>
          <div class="label">Son des notifications</div>
          <div class="hint">Permission requise, question, tâche terminée, erreur</div>
        </div>
        <input
          type="checkbox"
          class="switch"
          checked={settings.soundEnabled}
          onchange={(e) => run('set_sound', { enabled: e.currentTarget.checked })}
        />
      </label>
    </section>

    <section>
      <h2>Curseur</h2>
      <div class="stack">
        <div>
          <div class="label">Résistance au bord du notch</div>
          <div class="hint">
            {#if settings.resistanceAvailable}
              Le curseur bute contre le notch : il faut pousser pour entrer.
            {:else}
              Disponible uniquement sous Windows pour l'instant.
            {/if}
          </div>
        </div>
        <div class="segmented" role="radiogroup" aria-label="Résistance du curseur">
          {#each resistanceChoices as choice (choice.value)}
            <button
              role="radio"
              aria-checked={resistance === choice.value}
              class:selected={resistance === choice.value}
              disabled={!settings.resistanceAvailable}
              onclick={() => setResistance(choice.value)}
            >
              {choice.label}
            </button>
          {/each}
        </div>
      </div>
    </section>

    <section>
      <h2>Démarrage</h2>
      <label class="row">
        <div class="label">Lancer winotch au démarrage de l'ordinateur</div>
        <input
          type="checkbox"
          class="switch"
          checked={settings.autostart}
          onchange={(e) => run('set_autostart', { enabled: e.currentTarget.checked })}
        />
      </label>
    </section>

    <section>
      <h2>Avancé</h2>
      <form
        class="row"
        onsubmit={(e) => {
          e.preventDefault()
          void run('set_server_port', { port: Number(port) })
        }}
      >
        <div>
          <div class="label">Port du serveur local</div>
          <div class="hint">Claude Code est reconnecté automatiquement.</div>
        </div>
        <div class="field">
          <input type="number" min="1024" max="65535" bind:value={port} />
          <button disabled={port === String(settings.serverPort)}>Appliquer</button>
        </div>
      </form>
      <form
        class="row"
        onsubmit={(e) => {
          e.preventDefault()
          void run('set_session_timeout', { minutes: Number(timeout) })
        }}
      >
        <div>
          <div class="label">Oublier une session inactive après</div>
          <div class="hint">En minutes, si son terminal a été fermé brutalement.</div>
        </div>
        <div class="field">
          <input type="number" min="5" max="1440" bind:value={timeout} />
          <button disabled={timeout === String(settings.sessionTimeoutMinutes)}>Appliquer</button>
        </div>
      </form>
      <p class="hint path">Configuration : {settings.configDir}</p>
    </section>
  {/if}
</main>

<style>
  :global(html[data-window='settings']) {
    --bg: #f4f4f6;
    --card: #ffffff;
    --text: #1c1c1e;
    --muted: #6e6e73;
    --border: #e2e2e7;
    --accent: #0a84ff;
    --accent-text: #ffffff;
    --danger: #d70015;
    color-scheme: light;
    color: var(--text);
    font-size: 13px;
  }

  @media (prefers-color-scheme: dark) {
    :global(html[data-window='settings']) {
      --bg: #1c1c1e;
      --card: #2c2c2e;
      --text: #f2f2f7;
      --muted: #98989f;
      --border: #3a3a3c;
      --danger: #ff6961;
      color-scheme: dark;
    }
  }

  :global(html[data-window='settings'] body) {
    background: var(--bg);
  }

  main {
    max-width: 560px;
    margin: 0 auto;
    padding: 20px 20px 28px;
    display: grid;
    gap: 14px;
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  h1 {
    margin: 0;
    font-size: 20px;
  }

  h2 {
    margin: 0 0 10px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }

  section {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 14px 16px;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .row + .row,
  .separated {
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }

  .stack {
    display: grid;
    gap: 10px;
  }

  .label {
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .hint,
  .version {
    color: var(--muted);
    font-size: 12px;
    margin-top: 2px;
  }

  .path {
    margin: 14px 0 0;
    word-break: break-all;
  }

  .status {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted);
  }

  .status.on {
    background: #30d158;
  }

  .error {
    margin: 10px 0 0;
    color: var(--danger);
  }

  main > .error {
    margin: 0;
  }

  button {
    font: inherit;
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--card);
    color: var(--text);
    cursor: pointer;
    white-space: nowrap;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }

  .segmented {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    padding: 2px;
    border-radius: 8px;
    background: var(--bg);
    border: 1px solid var(--border);
  }

  .segmented button {
    border: none;
    background: transparent;
    padding: 6px 4px;
  }

  .segmented button.selected {
    background: var(--card);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.15);
    font-weight: 600;
  }

  .field {
    display: flex;
    gap: 6px;
  }

  select {
    font: inherit;
    max-width: 240px;
    padding: 5px 8px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
  }

  input[type='number'] {
    font: inherit;
    width: 76px;
    padding: 5px 8px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
  }

  .switch {
    appearance: none;
    flex-shrink: 0;
    width: 38px;
    height: 22px;
    margin: 0;
    border-radius: 11px;
    background: var(--border);
    position: relative;
    cursor: pointer;
    transition: background-color 150ms ease;
  }

  .switch::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.3);
    transition: transform 150ms ease;
  }

  .switch:checked {
    background: #30d158;
  }

  .switch:checked::after {
    transform: translateX(16px);
  }

  .switch:focus-visible,
  button:focus-visible,
  input:focus-visible,
  select:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
