<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Puzzle from '@lucide/svelte/icons/puzzle'
  import SettingsIcon from '@lucide/svelte/icons/settings'
  import type { ModuleUis } from '../modules'
  import './settings.css'

  let { moduleUis }: { moduleUis: ModuleUis } = $props()

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
    autostart: boolean
    modules: ModuleInfo[]
    configDir: string
  }

  // Mirrors `ModuleInfo` in src-tauri/src/settings.rs
  interface ModuleInfo {
    id: string
    name: string
    description: string
    enabled: boolean
    settings: unknown
  }

  // Categories of the left menu (DF-0007).
  type Page = 'general' | 'display' | 'modules' | 'about'
  const pages: { id: Page; label: string }[] = [
    { id: 'general', label: 'Général' },
    { id: 'display', label: 'Affichage' },
    { id: 'modules', label: 'Modules' },
    { id: 'about', label: 'À propos' },
  ]

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
  let page = $state<Page>('general')
  // Module whose settings page is open, within the Modules category.
  let openModuleId = $state<string | null>(null)
  const openModule = $derived(
    page === 'modules' ? settings?.modules.find((m) => m.id === openModuleId) : undefined,
  )

  function show(next: Page) {
    page = next
    openModuleId = null
    error = null
  }

  function setModuleEnabled(module: ModuleInfo, enabled: boolean) {
    void run('set_module_enabled', { id: module.id, enabled })
  }

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

<div class="layout">
  <nav aria-label="Catégories">
    <div class="brand">
      winotch
      {#if version}<span class="version">v{version}</span>{/if}
    </div>
    {#each pages as item (item.id)}
      <button
        class="nav-item"
        class:bottom={item.id === 'about'}
        class:current={page === item.id}
        aria-current={page === item.id ? 'page' : undefined}
        onclick={() => show(item.id)}
      >
        {item.label}
      </button>
    {/each}
  </nav>

  <main>
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    {#if settings}
      {#if page === 'general'}
        <h1>Général</h1>
        <section>
          <label class="row">
            <div class="label">Lancer winotch au démarrage de l'ordinateur</div>
            <input
              type="checkbox"
              class="switch"
              checked={settings.autostart}
              onchange={(e) => run('set_autostart', { enabled: e.currentTarget.checked })}
            />
          </label>
          <label class="row">
            <div>
              <div class="label">Son des notifications</div>
              <div class="hint">
                Quand un module demande ton attention : permission, question, fin de tâche, erreur
              </div>
            </div>
            <input
              type="checkbox"
              class="switch"
              checked={settings.soundEnabled}
              onchange={(e) => run('set_sound', { enabled: e.currentTarget.checked })}
            />
          </label>
        </section>
      {:else if page === 'display'}
        <h1>Affichage</h1>
        <section>
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
      {:else if page === 'modules' && openModule}
        {@const ModuleSettings = moduleUis[openModule.id]?.settings}
        <button class="back" onclick={() => (openModuleId = null)}>← Modules</button>
        <h1>{openModule.name}</h1>
        <section>
          <label class="row">
            <div>
              <div class="label">Activé</div>
              {#if openModule.description}<div class="hint">{openModule.description}</div>{/if}
            </div>
            <input
              type="checkbox"
              class="switch"
              checked={openModule.enabled}
              onchange={(e) => setModuleEnabled(openModule, e.currentTarget.checked)}
            />
          </label>
        </section>
        {#if openModule.enabled && ModuleSettings}
          <section>
            <ModuleSettings
              data={openModule.settings}
              call={(action, args) => run('module_call', { id: openModule.id, action, args })}
            />
          </section>
        {/if}
      {:else if page === 'modules'}
        <h1>Modules</h1>
        {#each settings.modules as module (module.id)}
          {@const ui = moduleUis[module.id]}
          {@const Icon = ui?.icon ?? Puzzle}
          {@const configurable = module.enabled && !!ui?.settings}
          <section class="module-card" class:disabled={!module.enabled}>
            <div class="module-icon"><Icon size={20} /></div>
            <div class="module-text">
              <div class="label">{module.name}</div>
              {#if module.description}<div class="hint">{module.description}</div>{/if}
            </div>
            <div class="actions">
              <input
                type="checkbox"
                class="switch"
                aria-label="Activer {module.name}"
                checked={module.enabled}
                onchange={(e) => setModuleEnabled(module, e.currentTarget.checked)}
              />
              <button
                class="icon"
                disabled={!configurable}
                title={configurable
                  ? `Paramètres de ${module.name}`
                  : module.enabled
                    ? 'Aucun réglage'
                    : 'Active le module pour le régler'}
                aria-label="Paramètres de {module.name}"
                onclick={() => (openModuleId = module.id)}
              >
                <SettingsIcon size={16} />
              </button>
            </div>
          </section>
        {/each}
      {:else}
        <h1>À propos</h1>
        <section>
          <div class="row">
            <div class="label">Version</div>
            <div>{version}</div>
          </div>
          <div class="row">
            <div>
              <div class="label">Dossier de configuration</div>
              <div class="hint path">{settings.configDir}</div>
            </div>
          </div>
          <div class="row">
            <div>
              <div class="label">Licence</div>
              <div class="hint">Le cœur de winotch est un logiciel libre sous licence GPL-3.0.</div>
            </div>
          </div>
        </section>
      {/if}
    {/if}
  </main>
</div>
