<script lang="ts">
  import Pause from '@lucide/svelte/icons/pause'
  import Play from '@lucide/svelte/icons/play'
  import Puzzle from '@lucide/svelte/icons/puzzle'
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw'
  import { cubicOut } from 'svelte/easing'
  import { fade } from 'svelte/transition'
  import type { ModuleUis } from '../modules'
  import {
    compactSection,
    mostUrgent,
    toneColor,
    type ActionIcon,
    type Card,
    type Edge,
    type Section,
    type Style,
  } from './content'

  const actionIcons: Record<ActionIcon, typeof Play> = {
    play: Play,
    pause: Pause,
    reset: RotateCcw,
  }

  let {
    cards,
    foldedWidth,
    duration,
    gap,
    allSections,
    note,
    moduleUis,
    notice,
    edge,
    style,
    expanded,
    movable,
    anchor,
    onclick,
    onaction,
    onenter,
    onleave,
    onpointerdown,
    onpointermove,
    onpointerup,
  }: {
    // The notch, then the other cards of the separate layout (DF-0011).
    cards: Card[]
    // Width of the closed notch: the other cards open from it, like the notch.
    foldedWidth: number
    // Opening / closing animation, in ms.
    duration: number
    // Space between two cards.
    gap: number
    // Every module, the compact notch picks one of them.
    allSections: Section[]
    note: string | null
    moduleUis: ModuleUis
    notice: string | null
    edge: Edge
    style: Style
    expanded: boolean
    movable: boolean
    anchor: number | null
    onclick: () => void
    // A button of an item: `itemId` is `<module id>:<item id>`.
    onaction: (itemId: string, actionId: string) => void
    onenter: () => void
    onleave: () => void
    onpointerdown: (e: PointerEvent) => void
    onpointermove: () => void
    onpointerup: () => void
  } = $props()

  // The compact notch shows a single module (DF-0011). Only some items have
  // a dot (a state, e.g. a Claude Code session), others just show text.
  const shown = $derived(compactSection(allSections))
  const top = $derived(shown && mostUrgent(shown.items))
  const dotted = $derived(shown?.items.filter((i) => i.dot) ?? [])
  const ShownIcon = $derived(shown ? iconOf(shown.module) : null)

  function iconOf(module: string) {
    return moduleUis[module]?.icon ?? Puzzle
  }
  const vertical = $derived(edge === 'left' || edge === 'right')

  // The cards are stacked: towards the inside of the screen on the top and
  // bottom edges, down along the edge on the left and right ones. The stack
  // is centered on the anchor along the edge, then kept inside the window: at
  // an end of the edge the notch opens towards the other end. Same rule as
  // `stack_in_window` in src-tauri/src/placement.rs (hit area).
  let windowWidth = $state(0)
  let windowHeight = $state(0)
  const notch = $derived(cards[0])
  const stackWidth = $derived(Math.max(...cards.map((c) => c.width)))
  const stackHeight = $derived(
    cards.reduce((sum, c) => sum + c.height, 0) + gap * (cards.length - 1),
  )
  // Each card has its final place from the start, from the attached side:
  // it unfolds there instead of being pushed by the cards before it.
  const offsets = $derived(
    cards.map((_, i) => cards.slice(0, i).reduce((sum, c) => sum + c.height + gap, 0)),
  )
  // A card opens like the notch: from the closed notch's width and from
  // nothing in height, in its place (from the attached side).
  function unfold(node: HTMLElement, { delay, from }: { delay: number; from: number }) {
    const width = node.offsetWidth
    const height = node.offsetHeight
    return {
      delay,
      duration,
      easing: cubicOut,
      css: (t: number) =>
        `overflow: hidden; width: ${from + (width - from) * t}px; height: ${height * t}px`,
    }
  }

  const start = $derived.by(() => {
    const length = vertical ? windowHeight : windowWidth
    const size = vertical ? stackHeight : stackWidth
    const center = anchor ?? length / 2
    return Math.min(Math.max(center - size / 2, 0), Math.max(length - size, 0))
  })
  const position = $derived(
    {
      top: `left: ${start}px; top: 0`,
      bottom: `left: ${start}px; bottom: 0`,
      left: `top: ${start}px; left: 0`,
      right: `top: ${start}px; right: 0`,
    }[edge],
  )

  // A notch rounds its inner corners only, the attached side stays square
  // (DF-0006); a pill rounds all four. Order: top-left, top-right,
  // bottom-right, bottom-left.
  const corners = $derived.by(() => {
    const r = notch.radius
    return style === 'pill'
      ? `${r}px`
      : {
          top: `0 0 ${r}px ${r}px`,
          bottom: `${r}px ${r}px 0 0`,
          left: `0 ${r}px ${r}px 0`,
          right: `${r}px 0 0 ${r}px`,
        }[edge]
  })
</script>

<svelte:window bind:innerWidth={windowWidth} bind:innerHeight={windowHeight} />

<!-- The modules shown in an open card, one part each, its icon first
     (DF-0011). `withNote`: the notch says so when no module is enabled.
     `height`: the card's final height, null to follow the card. -->
{#snippet sectionList(list: Section[], withNote: boolean, height: number | null)}
  <!-- A fixed height keeps the rows still while the card unfolds over them. -->
  <div
    class="list"
    class:centered={list.length === 0}
    style:height={height === null ? null : `${height}px`}
  >
    {#each list as section (section.module)}
      {@const Icon = iconOf(section.module)}
      {@const dottedRows = section.items.some((i) => i.dot)}
      <!-- One part per module, its icon first (DF-0011). -->
      <div class="section">
        <span class="module-icon"><Icon size={14} strokeWidth={2.25} /></span>
        <ul class="rows">
          {#each section.items as item (item.id)}
            <li class="row">
              {#if item.dot}
                <span
                  class="dot"
                  class:pulse={item.tone === 'active'}
                  style:background={toneColor(item.tone)}
                ></span>
              {:else if dottedRows}
                <!-- Keeps the texts aligned with the dotted rows. -->
                <span class="dot blank"></span>
              {/if}
              {#if item.title}<span class="title">{item.title}</span>{/if}
              <span class="label" class:lead={!item.title}>
                {item.label}{item.detail ? ` · ${item.detail}` : ''}
              </span>
              {#if item.actions.length > 0}
                <span class="actions">
                  {#each item.actions as action (action.id)}
                    {@const ButtonIcon = action.icon ? actionIcons[action.icon] : null}
                    <button
                      class="action"
                      class:text={!ButtonIcon}
                      title={action.label}
                      aria-label={action.label}
                      onclick={(e) => {
                        // Not an acknowledgement of the whole notch.
                        e.stopPropagation()
                        onaction(item.id, action.id)
                      }}
                    >
                      {#if ButtonIcon}
                        <ButtonIcon size={13} strokeWidth={2.5} />
                      {:else}
                        {action.label}
                      {/if}
                    </button>
                  {/each}
                </span>
              {/if}
            </li>
          {/each}
          <!-- A module without items still says how it is (ADR-0009). -->
          {#if section.note}
            <li class="row"><span class="label muted note">{section.note}</span></li>
          {/if}
        </ul>
      </div>
    {:else}
      {#if withNote && note}
        <span class="row"><span class="label muted note">{note}</span></span>
      {/if}
    {/each}
  </div>
{/snippet}

<!-- The window is larger than the notch: the stack of cards is glued to the
     attached side, it then grows towards the inside of the screen. Hovering
     the stack, gaps between cards included, keeps the notch open. -->
<div class="frame" style:--duration="{duration}ms">
  <div
    class="stack edge-{edge}"
    class:movable
    role="presentation"
    style={position}
    style:width="{stackWidth}px"
    style:height="{stackHeight}px"
    onmouseenter={onenter}
    onmouseleave={onleave}
  >
    <!-- Not a <button>: it holds the items' buttons. The window never takes
         the keyboard focus, so there is no keyboard access to provide. -->
    <div
      role="button"
      tabindex="-1"
      class="notch"
      class:vertical={vertical && !expanded && !notice}
      class:movable
      style:width="{notch.width}px"
      style:height="{notch.height}px"
      style:border-radius={corners}
      {onclick}
      onkeydown={(e) => e.key === 'Enter' && onclick()}
      {onpointerdown}
      {onpointermove}
      {onpointerup}
      onpointercancel={onpointerup}
    >
      {#if notice}
        <span class="row compact"><span class="label">{notice}</span></span>
      {:else if expanded}
        {@render sectionList(notch.sections, true, null)}
      {:else if vertical}
        <!-- Thin vertical notch: the shown module's dots, else its icon;
             details on hover. -->
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
        {:else if ShownIcon}
          <span class="module-icon"><ShownIcon size={14} strokeWidth={2.25} /></span>
        {/if}
      {:else if top}
        <span class="row compact">
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
          <span class="label">{top.label}</span>
          {#if top.title}<span class="muted">{top.title}</span>{/if}
        </span>
      {/if}
    </div>
    <!-- Separate layout: the other modules, one card each, rounded all round.
         Each unfolds in its place from the attached side, one after the
         other, once the notch has started opening. -->
    {#each cards.slice(1) as card, i (card.sections[0]?.module)}
      <div
        role="button"
        tabindex="-1"
        class="notch card"
        style:top={edge === 'bottom' ? null : `${offsets[i + 1]}px`}
        style:bottom={edge === 'bottom' ? `${offsets[i + 1]}px` : null}
        style:width="{card.width}px"
        style:height="{card.height}px"
        style:border-radius="{card.radius}px"
        in:unfold={{ delay: Math.round(duration * 0.28) * (i + 1), from: foldedWidth }}
        out:fade={{ duration: Math.round(duration / 2) }}
        {onclick}
        onkeydown={(e) => e.key === 'Enter' && onclick()}
      >
        {@render sectionList(card.sections, false, card.height)}
      </div>
    {/each}
  </div>
</div>

<style>
  .frame {
    position: relative;
    height: 100%;
  }

  /* The notch, then the other cards of the separate layout (DF-0011), each
     at its own place. It grows with the notch, so they stay centered on it. */
  .stack {
    position: absolute;
    transition:
      left var(--duration) ease,
      top var(--duration) ease,
      width var(--duration) ease,
      height var(--duration) ease;
  }

  .notch {
    all: unset;
    position: absolute;
    box-sizing: border-box;
    display: block;
    overflow: hidden;
    padding: 0 16px;
    background: var(--notch-bg);
    transition:
      width var(--duration) ease,
      height var(--duration) ease,
      border-radius var(--duration) ease;
  }

  /* Move mode: a grab cursor and a light outline say the notch can be dragged.
     No animation: the shape must follow the cursor at once. */
  .stack.movable,
  .notch.movable {
    transition: none;
  }

  /* Against the attached side; centered across it on the top and bottom
     edges, glued to it on the left and right ones. */
  .edge-top > .notch {
    top: 0;
  }

  .edge-bottom > .notch {
    bottom: 0;
  }

  .edge-left > .notch,
  .edge-right > .notch {
    top: 0;
  }

  .edge-top > .notch,
  .edge-bottom > .notch {
    left: 50%;
    transform: translateX(-50%);
  }

  .edge-left > .notch {
    left: 0;
  }

  .edge-right > .notch {
    right: 0;
  }

  .notch.movable {
    cursor: grab;
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.35);
  }

  .notch.movable:active {
    cursor: grabbing;
  }

  .notch.vertical {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .compact {
    height: 34px;
    justify-content: center;
  }

  /* Fills the open notch and centers its rows: a vertical notch never gets
     shorter than its compact shape, which leaves room around a short list.
     Past the window height it scrolls ("safe": the top stays reachable). */
  .list {
    box-sizing: border-box;
    height: 100%;
    padding: 8px 0 10px;
    display: grid;
    align-content: safe center;
    gap: 2px;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .section {
    display: flex;
    gap: 8px;
    min-width: 0;
  }

  /* Between two modules: a thin line. Its height (6 + 1 + 8 px) is
     SEPARATOR_HEIGHT in App.svelte. */
  .section + .section {
    margin-top: 6px;
    padding-top: 8px;
    border-top: 1px solid rgb(255 255 255 / 0.12);
  }

  .module-icon {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    height: 24px;
    opacity: 0.55;
  }

  .rows {
    flex: 1;
    min-width: 0;
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  .list .row {
    height: 24px;
  }

  .list .title {
    flex: 0 1 auto;
    font-weight: 600;
  }

  .list .label {
    margin-left: auto;
    opacity: 0.75;
  }

  .list .note,
  .list .lead {
    margin-left: 0;
  }

  .actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .action {
    all: unset;
    display: grid;
    place-items: center;
    min-width: 20px;
    height: 20px;
    border-radius: 6px;
    background: rgb(255 255 255 / 0.12);
    color: var(--notch-fg);
    cursor: pointer;
    transition: background-color 150ms ease;
  }

  .action.text {
    padding: 0 7px;
    font-size: 11px;
  }

  .action:hover {
    background: rgb(255 255 255 / 0.25);
  }

  .list.centered .row {
    justify-content: center;
  }

  .dot.blank {
    background: transparent;
  }

  .dots {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .dots.column {
    flex-direction: column;
    gap: 6px;
  }

  .dot {
    width: 8px;
    height: 8px;
    flex-shrink: 0;
    border-radius: 50%;
    transition: background-color 250ms ease;
  }

  .pulse {
    animation: pulse 1.2s ease-in-out infinite;
  }

  .label,
  .title,
  .muted {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .muted {
    opacity: 0.55;
  }

  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
</style>
