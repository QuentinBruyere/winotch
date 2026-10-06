<script lang="ts">
  import Puzzle from '@lucide/svelte/icons/puzzle'
  import type { ModuleUis } from '../modules'
  import Sections from './Sections.svelte'
  import { mostUrgent, ringing, toneColor, type Rect, type Section } from './content'

  // Space on each side of the closed pin's summary, part of `measured`.
  const PIN_PADDING = 12

  // A pinned module's mini-notch, next to the notch (DF-0012). Closed, it
  // shows the essentials only; hovered or alerting, the whole module.
  let {
    section,
    rect,
    corners,
    open,
    vertical,
    moduleUis,
    measured = $bindable(),
    onenter,
    onleave,
    onclick,
    onaction,
  }: {
    section: Section
    // Where and how big, from App.svelte (it also sets the window region).
    rect: Rect
    corners: string
    open: boolean
    // On the left / right edges of the screen: thin, dots or icon only.
    vertical: boolean
    moduleUis: ModuleUis
    // Width the closed pin needs for its summary, padding included; unset
    // until measured.
    measured?: number
    onenter: () => void
    onleave: () => void
    onclick: () => void
    onaction: (itemId: string, actionId: string) => void
  } = $props()

  const top = $derived(mostUrgent(section.items))
  const dotted = $derived(section.items.filter((i) => i.dot))
  const Icon = $derived(moduleUis[section.module]?.icon ?? Puzzle)

  let summaryWidth = $state(0)
  $effect(() => {
    if (!open && !vertical && summaryWidth > 0) {
      measured = Math.ceil(summaryWidth) + 2 * PIN_PADDING
    }
  })
</script>

<!-- Not a <button>: it holds the items' buttons once open. -->
<div
  role="button"
  tabindex="-1"
  class="pin"
  class:open
  class:vertical={vertical && !open}
  class:ringing={ringing([section])}
  style:left="{rect.x}px"
  style:top="{rect.y}px"
  style:width="{rect.width}px"
  style:height="{rect.height}px"
  style:border-radius={corners}
  style:padding={vertical && !open ? '0' : `0 ${open ? 16 : PIN_PADDING}px`}
  onmouseenter={onenter}
  onmouseleave={onleave}
  {onclick}
  onkeydown={(e) => e.key === 'Enter' && onclick()}
>
  {#if open}
    <Sections list={[section]} height={rect.height} {moduleUis} {onaction} />
  {:else if vertical}
    {#if dotted.length > 0}
      <span class="dots column">
        {#each dotted as item (item.id)}
          <span
            class="dot"
            class:pulse={item.tone === 'active'}
            style:background={toneColor(item.tone)}
          ></span>
        {/each}
      </span>
    {:else}
      <span class="module-icon"><Icon size={14} strokeWidth={2.25} /></span>
    {/if}
  {:else if top}
    <span class="summary" bind:offsetWidth={summaryWidth}>
      {#if dotted.length > 0}
        <span class="dots">
          {#each dotted as item (item.id)}
            <span
              class="dot"
              class:pulse={item.tone === 'active'}
              style:background={toneColor(item.tone)}
            ></span>
          {/each}
        </span>
      {/if}
      {#if top.quiet}
        <span class="module-icon"><Icon size={14} strokeWidth={2.25} /></span>
      {:else}
        <span class="label">{top.label}</span>
      {/if}
    </span>
  {/if}
</div>

<style>
  .pin {
    position: absolute;
    box-sizing: border-box;
    overflow: hidden;
    pointer-events: auto;
    background: var(--notch-bg);
    transition:
      left var(--duration) ease,
      top var(--duration) ease,
      width var(--duration) ease,
      height var(--duration) ease,
      border-radius var(--duration) ease;
  }

  /* Closed, its summary or icon is centered. */
  .pin:not(.open) {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* The essentials, at their natural width: measured to size the pin. */
  .summary {
    display: flex;
    align-items: center;
    gap: 6px;
    width: max-content;
    height: 100%;
    white-space: nowrap;
  }

  .summary .label {
    overflow: visible;
  }
</style>
