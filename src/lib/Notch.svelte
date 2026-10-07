<script lang="ts">
  import Puzzle from '@lucide/svelte/icons/puzzle'
  import { cubicOut } from 'svelte/easing'
  import { fade } from 'svelte/transition'
  import type { ModuleUis } from '../modules'
  import Sections from './Sections.svelte'
  import { icons } from './icons'
  import {
    compactSection,
    corners,
    mostUrgent,
    ringing,
    sliderPercent,
    toneColor,
    type Card,
    type Edge,
    type Section,
    type Style,
  } from './content'

  let {
    cards,
    start,
    foldedWidth,
    duration,
    gap,
    allSections,
    note,
    moduleUis,
    notice,
    spotlight,
    edge,
    style,
    expanded,
    movable,
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
    // Where the stack begins along the edge, from App.svelte (it also sets
    // the window region from the same numbers).
    start: number
    // Width of the closed notch: the other cards open from it, like the notch.
    foldedWidth: number
    // Opening / closing animation, in ms.
    duration: number
    // Space between two cards.
    gap: number
    // The notch's modules (pinned ones apart), the compact notch picks one.
    allSections: Section[]
    note: string | null
    moduleUis: ModuleUis
    notice: string | null
    // A module showing its value for a moment, even when quiet (DF-0013).
    spotlight: string | null
    edge: Edge
    style: Style
    expanded: boolean
    movable: boolean
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
  // The item's own icon (the volume's level), else the module's.
  const ShownIcon = $derived(
    top?.icon ? icons[top.icon] : shown ? iconOf(shown.module) : null,
  )
  const compactTitle = $derived(top ? (top.compact ? top.compact.title : top.title) : '')
  const quiet = $derived(!!top?.quiet && shown?.module !== spotlight)

  function iconOf(module: string) {
    return moduleUis[module]?.icon ?? Puzzle
  }
  const vertical = $derived(edge === 'left' || edge === 'right')

  // The cards are stacked: towards the inside of the screen on the top and
  // bottom edges, down along the edge on the left and right ones.
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

  const position = $derived(
    {
      top: `left: ${start}px; top: 0`,
      bottom: `left: ${start}px; bottom: 0`,
      left: `top: ${start}px; left: 0`,
      right: `top: ${start}px; right: 0`,
    }[edge],
  )

  const notchCorners = $derived(corners(edge, notch.radius, style === 'notch'))
  // A ringing module pulses the shape that shows it (DF-0009).
  const notchRinging = $derived(
    expanded ? ringing(notch.sections) : !!shown && ringing([shown]),
  )
</script>

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
      class:ringing={notchRinging}
      style:width="{notch.width}px"
      style:height="{notch.height}px"
      style:border-radius={notchCorners}
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
        <Sections list={notch.sections} {note} {moduleUis} {onaction} />
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
          {#if quiet && ShownIcon}
            <span class="module-icon"><ShownIcon size={14} strokeWidth={2.25} /></span>
          {:else}
            {#if top.slider}
              <!-- A slider's value as a small bar (e.g. the volume). -->
              <span class="meter">
                <span style:width="{sliderPercent(top.slider)}%"></span>
              </span>
            {/if}
            <span class="label">{top.compact?.label ?? top.label}</span>
          {/if}
          {#if compactTitle}<span class="muted">{compactTitle}</span>{/if}
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
        class:ringing={ringing(card.sections)}
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
        <Sections list={card.sections} height={card.height} {moduleUis} {onaction} />
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
    outline: 1px solid var(--notch-border);
    outline-offset: -1px;
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
    box-shadow: inset 0 0 0 1px var(--notch-outline);
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

  .meter {
    flex-shrink: 0;
    width: 64px;
    height: 4px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--notch-track);
  }

  .meter > span {
    display: block;
    height: 100%;
    background: var(--notch-fg);
    transition: width 150ms ease;
  }
</style>
