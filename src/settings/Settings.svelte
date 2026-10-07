<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import GripVertical from '@lucide/svelte/icons/grip-vertical'
  import PinIcon from '@lucide/svelte/icons/pin'
  import Puzzle from '@lucide/svelte/icons/puzzle'
  import SettingsIcon from '@lucide/svelte/icons/settings'
  import { flip } from 'svelte/animate'
  import { applyAppearance, type Appearance } from '../lib/theme'
  import type { ModuleUis } from '../modules'
  import './settings.css'

  let { moduleUis }: { moduleUis: ModuleUis } = $props()

  // Mirrors `Settings` in src-tauri/src/settings.rs
  type Strength = 'soft' | 'medium' | 'strong' | 'veryStrong' | 'impassable'
  type Edge = 'top' | 'bottom' | 'left' | 'right'
  type Style = 'notch' | 'pill'
  type ModuleLayout = 'joined' | 'separate'
  type NotchSpeed = 'slow' | 'normal' | 'fast'
  type PinSide = 'auto' | 'left' | 'right'
  interface Settings {
    soundEnabled: boolean
    hideInFullscreen: boolean
    edge: Edge
    style: Style
    gap: number
    screen: string | null
    screens: { id: string; label: string; primary: boolean }[]
    movable: boolean
    offCenter: boolean
    cursorResistance: boolean
    resistanceStrength: Strength
    resistanceAvailable: boolean
    autostart: boolean
    modules: ModuleInfo[]
    moduleLayout: ModuleLayout
    notchSpeed: NotchSpeed
    appearance: Appearance
    configDir: string
  }

  // Mirrors `ModuleInfo` in src-tauri/src/settings.rs
  interface ModuleInfo {
    id: string
    name: string
    description: string
    enabled: boolean
    pin: PinSide | null
    pinnable: boolean
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

  const appearanceChoices: { value: Appearance; label: string }[] = [
    { value: 'system', label: 'Système' },
    { value: 'light', label: 'Clair' },
    { value: 'dark', label: 'Sombre' },
  ]

  const speedChoices: { value: NotchSpeed; label: string }[] = [
    { value: 'slow', label: 'Lente' },
    { value: 'normal', label: 'Normale' },
    { value: 'fast', label: 'Rapide' },
  ]

  const layoutChoices: { value: ModuleLayout; label: string }[] = [
    { value: 'joined', label: 'Collés' },
    { value: 'separate', label: 'Décollés' },
  ]

  const styleChoices: { value: Style; label: string }[] = [
    { value: 'notch', label: 'Notch' },
    { value: 'pill', label: 'Pilule' },
  ]

  // Shown while the slider moves, saved when it is released.
  let gap = $state(10)
  $effect(() => {
    if (settings) gap = settings.gap
  })

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
    { value: 'veryStrong', label: 'Très forte' },
    { value: 'impassable', label: 'Infranchissable' },
  ]

  let settings = $state<Settings | null>(null)
  $effect(() => {
    if (settings) applyAppearance(settings.appearance)
  })
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

  // Pinned next to the notch (DF-0012); on the left / right edges of the
  // screen, left and right mean above and below.
  const pinSideChoices: { value: PinSide; label: string }[] = [
    { value: 'auto', label: 'Auto' },
    { value: 'left', label: 'Gauche' },
    { value: 'right', label: 'Droite' },
  ]

  function setModulePin(module: ModuleInfo, pin: PinSide | null) {
    void run('set_module_pin', { id: module.id, pin })
  }

  // Reordering the modules by dragging their handle (DF-0011). The grabbed
  // card follows the pointer, the others slide out of its way; the list
  // itself only changes on release, then the order is saved.
  interface Drag {
    from: number
    to: number
    // How far the grabbed card moved from its place, in px.
    dy: number
    startY: number
    // Layout of the cards when the drag started, in list order.
    tops: number[]
    heights: number[]
    // Room the grabbed card takes: its height plus the gap after it.
    span: number
  }
  let drag = $state<Drag | null>(null)
  // The new order, shown until the saved settings come back.
  let localOrder = $state<string[] | null>(null)
  const cards = $state<Record<string, HTMLElement>>({})
  const orderedModules = $derived.by(() => {
    const modules = settings?.modules ?? []
    if (!localOrder) return modules
    return localOrder.flatMap((id) => modules.find((m) => m.id === id) ?? [])
  })

  // Pointer position in the scrolling area, so scrolling while dragging counts.
  function pointerY(e: PointerEvent, card: HTMLElement) {
    return e.clientY + ((card.offsetParent as HTMLElement | null)?.scrollTop ?? 0)
  }

  function startReorder(e: PointerEvent, id: string) {
    if (e.button !== 0 || drag) return
    const elements = orderedModules.map((m) => cards[m.id])
    const from = orderedModules.findIndex((m) => m.id === id)
    const card = elements[from]
    if (!card || elements.some((el) => !el)) return
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
    const gap = parseFloat(getComputedStyle(card.offsetParent ?? card).rowGap) || 0
    drag = {
      from,
      to: from,
      dy: 0,
      startY: pointerY(e, card),
      tops: elements.map((el) => el.offsetTop),
      heights: elements.map((el) => el.offsetHeight),
      span: card.offsetHeight + gap,
    }
  }

  function reorder(e: PointerEvent) {
    if (!drag) return
    const { from, tops, heights } = drag
    const last = tops.length - 1
    const card = cards[orderedModules[from].id]
    // Kept within the list.
    const dy = Math.min(
      Math.max(pointerY(e, card) - drag.startY, tops[0] - tops[from]),
      tops[last] + heights[last] - (tops[from] + heights[from]),
    )
    // A card passes another once its leading edge crosses the other's middle:
    // its bottom going down, its top going up. With the middles, the last
    // place could never be reached (the card stops at the end of the list).
    const top = tops[from] + dy
    const bottom = top + heights[from]
    drag.dy = dy
    drag.to = tops.filter((t, i) => {
      const middle = t + heights[i] / 2
      return i < from ? middle < top : i > from && middle < bottom
    }).length
  }

  // Where each card is drawn while dragging, relative to its place.
  function shift(index: number): number {
    if (!drag) return 0
    const { from, to } = drag
    if (index === from) return drag.dy
    if (from < index && index <= to) return -drag.span
    if (to <= index && index < from) return drag.span
    return 0
  }

  async function endReorder() {
    if (!drag) return
    const { from, to } = drag
    const order = orderedModules.map((m) => m.id)
    const [moved] = order.splice(from, 1)
    order.splice(to, 0, moved)
    // Same update: the list takes its new order as the shifts disappear, and
    // the grabbed card glides from where it was dropped to its place.
    drag = null
    if (from === to) return
    localOrder = order
    await run('set_module_order', { order })
    localOrder = null
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
    if (!drag) localOrder = null
  }

  async function run(command: string, args: Record<string, unknown> = {}) {
    error = null
    try {
      await invoke(command, args)
    } catch (e) {
      error = String(e)
    }
  }

  // Position of the speed slider, follows the saved setting.
  let speedIndex = $state(1)
  $effect(() => {
    if (settings) {
      speedIndex = Math.max(
        0,
        speedChoices.findIndex((c) => c.value === settings?.notchSpeed),
      )
    }
  })

  // Position of the notched slider, follows the saved setting.
  let resistanceIndex = $state(0)
  $effect(() => {
    resistanceIndex = Math.max(
      0,
      resistanceChoices.findIndex((c) => c.value === resistance),
    )
  })

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
              <div class="label">Apparence</div>
              <div class="hint">
                Notch et paramètres clairs ou sombres. Système suit le mode choisi dans Windows.
              </div>
            </div>
            <div
              class="segmented"
              style:grid-template-columns="repeat(3, 1fr)"
              role="radiogroup"
              aria-label="Apparence"
            >
              {#each appearanceChoices as choice (choice.value)}
                <button
                  role="radio"
                  aria-checked={settings.appearance === choice.value}
                  class:selected={settings.appearance === choice.value}
                  onclick={() => run('set_appearance', { appearance: choice.value })}
                >
                  {choice.label}
                </button>
              {/each}
            </div>
          </div>
          <div class="stack separated">
            <div>
              <div class="label">Forme</div>
              <div class="hint">
                Notch collé au bord de l'écran, ou pilule détachée du bord, façon Dynamic Island.
              </div>
            </div>
            <div
              class="segmented"
              style:grid-template-columns="repeat(2, 1fr)"
              role="radiogroup"
              aria-label="Forme"
            >
              {#each styleChoices as choice (choice.value)}
                <button
                  role="radio"
                  aria-checked={settings.style === choice.value}
                  class:selected={settings.style === choice.value}
                  onclick={() => run('set_style', { style: choice.value })}
                >
                  {choice.label}
                </button>
              {/each}
            </div>
          </div>
          {#if settings.style === 'pill'}
            <label class="row separated">
              <div>
                <div class="label">Écart avec le bord</div>
                <div class="hint">{gap} px</div>
              </div>
              <input
                type="range"
                min="4"
                max="40"
                step="1"
                bind:value={gap}
                onchange={() => run('set_gap', { gap })}
              />
            </label>
          {/if}
          <div class="stack separated">
            <div>
              <div class="label">Vitesse d'ouverture</div>
              <div class="hint">Quand le notch s'ouvre au survol ou lors d'une alerte, et se referme.</div>
            </div>
            <!-- Notched slider, like the cursor resistance one. -->
            <div class="notched">
              <input
                type="range"
                min="0"
                max={speedChoices.length - 1}
                step="1"
                aria-label="Vitesse d'ouverture"
                aria-valuetext={speedChoices[speedIndex].label}
                bind:value={speedIndex}
                onchange={() => run('set_notch_speed', { speed: speedChoices[speedIndex].value })}
              />
              <div class="stops">
                {#each speedChoices as choice, i (choice.value)}
                  <button
                    class:selected={speedIndex === i}
                    style:left="{(i / (speedChoices.length - 1)) * 100}%"
                    onclick={() => {
                      speedIndex = i
                      void run('set_notch_speed', { speed: choice.value })
                    }}
                  >
                    {choice.label}
                  </button>
                {/each}
              </div>
            </div>
          </div>
          <div class="stack separated">
            <div>
              <div class="label">Modules</div>
              <div class="hint">
                Quand plusieurs modules s'affichent : dans le notch, séparés par un trait, ou
                chacun dans sa carte, sous le notch.
              </div>
            </div>
            <div
              class="segmented"
              style:grid-template-columns="repeat(2, 1fr)"
              role="radiogroup"
              aria-label="Modules"
            >
              {#each layoutChoices as choice (choice.value)}
                <button
                  role="radio"
                  aria-checked={settings.moduleLayout === choice.value}
                  class:selected={settings.moduleLayout === choice.value}
                  onclick={() => run('set_module_layout', { layout: choice.value })}
                >
                  {choice.label}
                </button>
              {/each}
            </div>
          </div>
          <div class="stack separated">
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
                {#if !settings.resistanceAvailable}
                  Disponible uniquement sous Windows pour l'instant.
                {:else if resistance === 'impassable'}
                  Le curseur ne peut plus entrer : le notch ne s'ouvre plus au survol ni ne se
                  clique, seules les alertes l'ouvrent. Le mode déplacement suspend la résistance.
                {:else}
                  Le curseur bute contre le notch : il faut pousser pour entrer.
                {/if}
              </div>
            </div>
            <!-- Notched slider: one stop per preset, labels under the stops. -->
            <div class="notched">
              <input
                type="range"
                min="0"
                max={resistanceChoices.length - 1}
                step="1"
                aria-label="Résistance du curseur"
                aria-valuetext={resistanceChoices[resistanceIndex].label}
                disabled={!settings.resistanceAvailable}
                bind:value={resistanceIndex}
                onchange={() => setResistance(resistanceChoices[resistanceIndex].value)}
              />
              <div class="stops">
                {#each resistanceChoices as choice, i (choice.value)}
                  <button
                    class:selected={resistanceIndex === i}
                    style:left="{(i / (resistanceChoices.length - 1)) * 100}%"
                    disabled={!settings.resistanceAvailable}
                    onclick={() => {
                      resistanceIndex = i
                      setResistance(choice.value)
                    }}
                  >
                    {choice.label}
                  </button>
                {/each}
              </div>
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
        <p class="intro">
          Fais glisser un module par sa poignée pour choisir l'ordre d'affichage dans le notch : le
          premier est le plus près du bord. L'épingle place un module dans son propre petit notch,
          à côté du principal.
        </p>
        {#each orderedModules as module, index (module.id)}
          {@const ui = moduleUis[module.id]}
          {@const Icon = ui?.icon ?? Puzzle}
          {@const configurable = module.enabled && !!ui?.settings}
          <section
            class="module-card"
            class:disabled={!module.enabled}
            class:dragged={drag?.from === index}
            class:sliding={drag !== null && drag.from !== index}
            style:transform={drag ? `translateY(${shift(index)}px)` : undefined}
            bind:this={cards[module.id]}
            animate:flip={{ duration: 150 }}
          >
            <button
              class="grip"
              title="Faire glisser pour changer l'ordre"
              aria-label="Changer la place de {module.name}"
              onpointerdown={(e) => startReorder(e, module.id)}
              onpointermove={reorder}
              onpointerup={endReorder}
              onpointercancel={endReorder}
            >
              <GripVertical size={16} />
            </button>
            <div class="module-icon"><Icon size={20} /></div>
            <div class="module-text">
              <div class="label">{module.name}</div>
              {#if module.description}<div class="hint">{module.description}</div>{/if}
            </div>
            <div class="actions">
              {#if module.pin && module.pinnable}
                <select
                  aria-label="Côté de l'épingle de {module.name}"
                  title={settings.edge === 'left' || settings.edge === 'right'
                    ? 'Gauche : au-dessus du notch, Droite : au-dessous'
                    : 'Côté du notch où se place l’épingle'}
                  value={module.pin}
                  onchange={(e) => setModulePin(module, e.currentTarget.value as PinSide)}
                >
                  {#each pinSideChoices as choice (choice.value)}
                    <option value={choice.value}>{choice.label}</option>
                  {/each}
                </select>
              {/if}
              <button
                class="icon pin"
                class:pinned={!!module.pin && module.pinnable}
                disabled={!module.pinnable}
                aria-pressed={!!module.pin && module.pinnable}
                title={!module.enabled
                  ? 'Active le module pour l’épingler'
                  : !module.pinnable
                    ? 'Le premier module est toujours dans le notch'
                    : module.pin
                      ? 'Détacher du notch'
                      : 'Épingler à côté du notch'}
                aria-label="Épingler {module.name}"
                onclick={() => setModulePin(module, module.pin ? null : 'auto')}
              >
                <PinIcon size={16} />
              </button>
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
