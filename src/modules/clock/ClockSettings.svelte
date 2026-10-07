<script lang="ts">
  import { t, type Key } from '../../lib/i18n.svelte'
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

  const showChoices: { value: Show; key: Key }[] = [
    { value: 'time_and_date', key: 'clock.show.time_and_date' },
    { value: 'time', key: 'clock.show.time' },
    { value: 'date', key: 'clock.show.date' },
  ]
  const hourChoices: { value: boolean; key: Key }[] = [
    { value: false, key: 'clock.hour.24' },
    { value: true, key: 'clock.hour.12' },
  ]
  // Examples of each style, in the shown language.
  const dateChoices: { value: DateStyle; key: Key }[] = [
    { value: 'long', key: 'clock.example.long' },
    { value: 'short', key: 'clock.example.short' },
    { value: 'numeric', key: 'clock.example.numeric' },
  ]

  const showsTime = $derived(data.show !== 'date')
  const showsDate = $derived(data.show !== 'time')

  // The module takes its whole settings object at once.
  function update(change: Partial<Data>) {
    void call('update', { ...data, ...change })
  }
</script>

<div class="stack">
  <div class="label">{t('clock.show')}</div>
  <div class="segmented" style:grid-template-columns="repeat(3, 1fr)" role="radiogroup" aria-label={t('clock.show')}>
    {#each showChoices as choice (choice.value)}
      <button
        role="radio"
        aria-checked={data.show === choice.value}
        class:selected={data.show === choice.value}
        onclick={() => update({ show: choice.value })}
      >
        {t(choice.key)}
      </button>
    {/each}
  </div>
</div>

{#if showsTime}
  <div class="stack separated">
    <div class="label">{t('clock.hour_format')}</div>
    <div class="segmented" style:grid-template-columns="repeat(2, 1fr)" role="radiogroup" aria-label={t('clock.hour_format')}>
      {#each hourChoices as choice (choice.value)}
        <button
          role="radio"
          aria-checked={data.hour12 === choice.value}
          class:selected={data.hour12 === choice.value}
          onclick={() => update({ hour12: choice.value })}
        >
          {t(choice.key)}
        </button>
      {/each}
    </div>
  </div>
  <label class="row separated">
    <div class="label">{t('clock.seconds')}</div>
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
    <div class="label">{t('clock.date_format')}</div>
    <div class="segmented" style:grid-template-columns="repeat(3, 1fr)" role="radiogroup" aria-label={t('clock.date_format')}>
      {#each dateChoices as choice (choice.value)}
        <button
          role="radio"
          aria-checked={data.dateStyle === choice.value}
          class:selected={data.dateStyle === choice.value}
          onclick={() => update({ dateStyle: choice.value })}
        >
          {t(choice.key)}
        </button>
      {/each}
    </div>
  </div>
{/if}
