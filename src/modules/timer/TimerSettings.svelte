<script lang="ts">
  import type { ModuleSettingsProps } from '..'

  // Mirrors `Settings` in src-tauri/src/modules/timer/mod.rs
  interface Data {
    defaultMinutes: number
    favorites: number[]
  }

  let { data: raw, call }: ModuleSettingsProps = $props()
  const data = $derived(raw as Data)

  // Text fields are edited locally and applied on demand.
  let duration = $state('')
  let favorites = $state('')
  $effect(() => {
    duration = String(data.defaultMinutes)
    favorites = data.favorites.join(', ')
  })

  // "5, 15, 25" -> [5, 15, 25]; anything else is left for the module to refuse.
  const parsedFavorites = $derived(
    favorites
      .split(/[\s,;]+/)
      .filter(Boolean)
      .map(Number),
  )
</script>

<form
  class="row separated"
  onsubmit={(e) => {
    e.preventDefault()
    void call('update', { ...data, defaultMinutes: Number(duration) })
  }}
>
  <div>
    <div class="label">Durée par défaut</div>
    <div class="hint">En minutes, celle du minuteur à son lancement.</div>
  </div>
  <div class="field">
    <input type="number" min="1" max="600" bind:value={duration} />
    <button disabled={duration === String(data.defaultMinutes)}>Appliquer</button>
  </div>
</form>
<form
  class="row separated"
  onsubmit={(e) => {
    e.preventDefault()
    void call('update', { ...data, favorites: parsedFavorites })
  }}
>
  <div>
    <div class="label">Durées favorites</div>
    <div class="hint">En minutes, séparées par des virgules (5 au plus). Un clic dans le notch les choisit.</div>
  </div>
  <div class="field">
    <input type="text" placeholder="5, 15, 25" bind:value={favorites} />
    <button disabled={favorites === data.favorites.join(', ')}>Appliquer</button>
  </div>
</form>
