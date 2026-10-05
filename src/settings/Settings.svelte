<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { settingsComponents } from '../modules'
  import './settings.css'

  // Mirrors `Settings` in src-tauri/src/settings.rs
  type Strength = 'soft' | 'medium' | 'strong'
  type Edge = 'top' | 'bottom' | 'left' | 'right'
  interface Settings {
    soundEnabled: boolean
    hideInFullscreen: boolean
    edge: Edge
    screen: string | null
    screens: { id: string; label: string; primary: boolean }[]
    movable: boolean
    offCenter: boolean
    cursorResistance: boolean
    resistanceStrength: Strength
    resistanceAvailable: boolean
    serverPort: number
    sessionTimeoutMinutes: number
    autostart: boolean
    modules: { id: string; name: string; enabled: boolean; settings: unknown }[]
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
    {#each settings.modules as module (module.id)}
      {@const ModuleSettings = settingsComponents[module.id]}
      <section>
        <h2>{module.name}</h2>
        <label class="row">
          <div class="label">Activé</div>
          <input
            type="checkbox"
            class="switch"
            checked={module.enabled}
            onchange={(e) =>
              run('set_module_enabled', { id: module.id, enabled: e.currentTarget.checked })}
          />
        </label>
        {#if module.enabled && ModuleSettings}
          <ModuleSettings
            data={module.settings}
            call={(action, args) => run('module_call', { id: module.id, action, args })}
          />
        {/if}
      </section>
    {/each}

    <section>
      <h2>Affichage</h2>
      <div class="stack">
        <div>
          <div class="label">Position du notch</div>
          <div class="hint">
            Changer de bord recentre le notch. À gauche et à droite, le
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
      <label class="row separated">
        <div>
          <div class="label">Déplacer le notch</div>
          <div class="hint">
            Fais-le glisser le long de son bord ; il s'aimante au centre et aux quarts. Se
            désactive à la fermeture des paramètres.
          </div>
        </div>
        <input
          type="checkbox"
          class="switch"
          checked={settings.movable}
          onchange={(e) => run('set_movable', { movable: e.currentTarget.checked })}
        />
      </label>
      {#if settings.offCenter}
        <div class="row">
          <div class="hint">Le notch n'est plus centré sur son bord.</div>
          <button onclick={() => run('recenter')}>Recentrer</button>
        </div>
      {/if}
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
      <p class="hint path">Configuration : {settings.configDir}</p>
    </section>
  {/if}
</main>
