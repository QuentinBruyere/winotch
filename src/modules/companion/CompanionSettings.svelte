<script lang="ts">
  import Palette from '@lucide/svelte/icons/palette'
  import type { CompanionSize } from '../../lib/content'
  import { t, type Key } from '../../lib/i18n.svelte'
  import type { ModuleSettingsProps } from '..'

  // Mirrors `DeskCompanion::settings` in
  // src-tauri/src/modules/companion/mod.rs (DF-0024).
  interface Data {
    companion: string
    color: string | null
    dimmed: boolean
    size: CompanionSize
    companions: { id: string; name: string }[]
    colors: { id: string; name: string }[]
  }

  let { data: raw, call }: ModuleSettingsProps = $props()
  const data = $derived(raw as Data)
  // A color picked with the system's picker rather than from the palette.
  const custom = $derived(data.color?.startsWith('#') ? data.color : null)

  const sizeChoices: { value: CompanionSize; key: Key }[] = [
    { value: 'small', key: 'claude-code.companion_size.small' },
    { value: 'medium', key: 'claude-code.companion_size.medium' },
    { value: 'large', key: 'claude-code.companion_size.large' },
  ]
</script>

<div class="stack separated">
  <div class="label">{t('companion.module.choice')}</div>
  <div
    class="segmented"
    style:grid-template-columns="repeat({data.companions.length}, 1fr)"
    role="radiogroup"
    aria-label={t('companion.module.choice')}
  >
    {#each data.companions as choice (choice.id)}
      <button
        role="radio"
        aria-checked={data.companion === choice.id}
        class:selected={data.companion === choice.id}
        onclick={() => call('set_companion', { id: choice.id })}
      >
        {choice.name}
      </button>
    {/each}
  </div>
</div>
<div class="stack separated">
  <div class="label">{t('companion.module.color')}</div>
  <div class="swatches" role="radiogroup" aria-label={t('companion.module.color')}>
    <button
      class="swatch default"
      role="radio"
      aria-checked={data.color === null}
      class:selected={data.color === null}
      onclick={() => call('set_color', { color: null })}
    >
      {t('companion.module.default_color')}
    </button>
    {#each data.colors as color (color.id)}
      <button
        class="swatch"
        role="radio"
        aria-checked={data.color === color.id}
        aria-label={color.name}
        title={color.name}
        class:selected={data.color === color.id}
        style:background="var(--companion-{color.id})"
        onclick={() => call('set_color', { color: color.id })}
      ></button>
    {/each}
    <!-- Any color: the system's picker. -->
    <label
      class="swatch picker"
      class:selected={custom !== null}
      style:background={custom}
      title={t('settings.color.custom')}
    >
      <input
        type="color"
        value={custom ?? '#ffffff'}
        onchange={(e) => call('set_color', { color: e.currentTarget.value })}
      />
      <Palette size={14} />
    </label>
  </div>
</div>
<label class="row separated">
  <div class="label">{t('companion.module.dimmed')}</div>
  <input
    type="checkbox"
    class="switch"
    checked={data.dimmed}
    onchange={(e) => call('set_dimmed', { dimmed: e.currentTarget.checked })}
  />
</label>
<div class="stack separated">
  <div class="label">{t('companion.module.size')}</div>
  <div
    class="segmented"
    style:grid-template-columns="repeat(3, 1fr)"
    role="radiogroup"
    aria-label={t('companion.module.size')}
  >
    {#each sizeChoices as choice (choice.value)}
      <button
        role="radio"
        aria-checked={data.size === choice.value}
        class:selected={data.size === choice.value}
        onclick={() => call('set_size', { size: choice.value })}
      >
        {t(choice.key)}
      </button>
    {/each}
  </div>
</div>
