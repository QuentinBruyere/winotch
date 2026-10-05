<script lang="ts">
  import type { ModuleSettingsProps } from '..'

  // Mirrors `Settings` in src-tauri/src/modules/clock/format.rs
  type Show = 'time_and_date' | 'time' | 'date'
  type DateStyle = 'long' | 'short' | 'numeric'
  interface Data {
    show: Show
    hour12: boolean
    seconds: boolean
    dateStyle: DateStyle
  }

  let { data: raw, call }: ModuleSettingsProps = $props()
  const data = $derived(raw as Data)

  const showChoices: { value: Show; label: string }[] = [
    { value: 'time_and_date', label: 'Heure et date' },
    { value: 'time', label: 'Heure' },
    { value: 'date', label: 'Date' },
  ]
  const hourChoices: { value: boolean; label: string }[] = [
    { value: false, label: '24 h (14:32)' },
    { value: true, label: '12 h (2:32 PM)' },
  ]
  const dateChoices: { value: DateStyle; label: string }[] = [
    { value: 'long', label: 'lundi 5 octobre' },
    { value: 'short', label: 'lun. 5 oct.' },
    { value: 'numeric', label: '05/10/2026' },
  ]

  const showsTime = $derived(data.show !== 'date')
  const showsDate = $derived(data.show !== 'time')

  // The module takes its whole settings object at once.
  function update(change: Partial<Data>) {
    void call('update', { ...data, ...change })
  }
</script>

<div class="stack">
  <div class="label">Affichage</div>
  <div class="segmented" style:grid-template-columns="repeat(3, 1fr)" role="radiogroup" aria-label="Affichage">
    {#each showChoices as choice (choice.value)}
      <button
        role="radio"
        aria-checked={data.show === choice.value}
        class:selected={data.show === choice.value}
        onclick={() => update({ show: choice.value })}
      >
        {choice.label}
      </button>
    {/each}
  </div>
</div>

{#if showsTime}
  <div class="stack separated">
    <div class="label">Format de l'heure</div>
    <div class="segmented" style:grid-template-columns="repeat(2, 1fr)" role="radiogroup" aria-label="Format de l'heure">
      {#each hourChoices as choice (choice.value)}
        <button
          role="radio"
          aria-checked={data.hour12 === choice.value}
          class:selected={data.hour12 === choice.value}
          onclick={() => update({ hour12: choice.value })}
        >
          {choice.label}
        </button>
      {/each}
    </div>
  </div>
  <label class="row separated">
    <div class="label">Afficher les secondes</div>
    <input
      type="checkbox"
      class="switch"
      checked={data.seconds}
      onchange={(e) => update({ seconds: e.currentTarget.checked })}
    />
  </label>
{/if}

{#if showsDate}
  <div class="stack separated">
    <div class="label">Format de la date</div>
    <div class="segmented" style:grid-template-columns="repeat(3, 1fr)" role="radiogroup" aria-label="Format de la date">
      {#each dateChoices as choice (choice.value)}
        <button
          role="radio"
          aria-checked={data.dateStyle === choice.value}
          class:selected={data.dateStyle === choice.value}
          onclick={() => update({ dateStyle: choice.value })}
        >
          {choice.label}
        </button>
      {/each}
    </div>
  </div>
{/if}
