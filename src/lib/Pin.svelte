<script lang="ts">
  import Puzzle from '@lucide/svelte/icons/puzzle'
  import type { ModuleUis } from '../modules'
  import Cover from './Cover.svelte'
  import Marquee from './Marquee.svelte'
  import Sections from './Sections.svelte'
  import { icons } from './icons'
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
    spotlit,
    ontextreadtime,
    vertical,
    moduleUis,
    measured = $bindable(),
    onenter,
    onleave,
    onclick,
    onaction,
    onmenu,
  }: {
    section: Section
    // Where and how big, from App.svelte (it also sets the window region).
    rect: Rect
    corners: string
    open: boolean
    // Just changed (the volume keys): its value for a moment, even when quiet.
    spotlit: boolean
    // How long its compact text needs to be read once (Marquee).
    ontextreadtime: (module: string, ms: number) => void
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
    // Right click on the pin (its module's id).
    onmenu: (module: string) => void
  } = $props()

  const top = $derived(mostUrgent(section.items))
  const compactLabel = $derived(top ? (top.compact?.label ?? top.label) : '')
  // A spotlight shows the text of a quiet item, if it has one: an item with
  // nothing to say (a media module with nothing playing) keeps its icon.
  const quiet = $derived(!!top?.quiet && (!spotlit || !compactLabel))
  const dotted = $derived(section.items.filter((i) => i.dot))
  // The item's own icon (the volume's level), else the module's.
  const Icon = $derived(
    top?.icon ? icons[top.icon] : (moduleUis[section.module]?.icon ?? Puzzle),
  )

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
  oncontextmenu={(e) => {
    e.preventDefault()
    onmenu(section.module)
  }}
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
      {#if quiet && top.image !== null}
        <Cover src={top.image} icon={moduleUis[section.module]?.icon ?? Puzzle} dimmed={top.dimmed} />
      {:else if quiet}
        <span class="module-icon"><Icon size={14} strokeWidth={2.25} /></span>
      {:else}
        {#if top.image !== null}
          <Cover src={top.image} icon={moduleUis[section.module]?.icon ?? Puzzle} dimmed={top.dimmed} />
        {/if}
        {#if compactLabel}
          <Marquee
            text={compactLabel}
            scroll={top.scroll}
            dimmed={top.dimmed}
            onturn={(ms) => ontextreadtime(section.module, ms)}
          />
        {/if}
        {#if top.compact?.title}<span class="muted">{top.compact.title}</span>{/if}
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
    outline: 1px solid var(--notch-border);
    outline-offset: -1px;
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

  /* A long text (a song title) stops the pin from growing too wide. */
  .summary :global(.label) {
    max-width: 180px;
  }
</style>
