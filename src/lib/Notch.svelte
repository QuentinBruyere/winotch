<script lang="ts">
  import Pause from '@lucide/svelte/icons/pause'
  import Play from '@lucide/svelte/icons/play'
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw'
  import {
    mostUrgent,
    toneColor,
    type ActionIcon,
    type Edge,
    type Item,
    type Style,
  } from './content'

  const actionIcons: Record<ActionIcon, typeof Play> = {
    play: Play,
    pause: Pause,
    reset: RotateCcw,
  }

  let {
    items,
    notes,
    notice,
    edge,
    style,
    expanded,
    width,
    height,
    radius,
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
    items: Item[]
    notes: string[]
    notice: string | null
    edge: Edge
    style: Style
    expanded: boolean
    width: number
    height: number
    radius: number
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

  const top = $derived(mostUrgent(items))
  // Only some items have a dot (a state), others just show text (the time).
  const dotted = $derived(items.filter((i) => i.dot))
  const vertical = $derived(edge === 'left' || edge === 'right')

  // Every shape is centered on the anchor along the edge, then kept inside the
  // window: at an end of the edge the notch opens towards the other end. Same
  // rule as `shape_in_window` in src-tauri/src/placement.rs (hit area).
  let windowWidth = $state(0)
  let windowHeight = $state(0)
  const start = $derived.by(() => {
    const length = vertical ? windowHeight : windowWidth
    const size = vertical ? height : width
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
  const corners = $derived(
    style === 'pill'
      ? `${radius}px`
      : {
          top: `0 0 ${radius}px ${radius}px`,
          bottom: `${radius}px ${radius}px 0 0`,
          left: `0 ${radius}px ${radius}px 0`,
          right: `${radius}px 0 0 ${radius}px`,
        }[edge],
  )
</script>

<svelte:window bind:innerWidth={windowWidth} bind:innerHeight={windowHeight} />

<!-- The window is larger than the notch: the shape is glued to the attached
     side, it then grows towards the inside of the screen. -->
<div class="frame">
  <!-- Not a <button>: it holds the items' buttons. The window never takes
       the keyboard focus, so there is no keyboard access to provide. -->
  <div
    role="button"
    tabindex="-1"
    class="notch"
    class:vertical={vertical && !expanded && !notice}
    class:movable
    style={position}
    style:width="{width}px"
    style:height="{height}px"
    style:border-radius={corners}
    {onclick}
    onkeydown={(e) => e.key === 'Enter' && onclick()}
    onmouseenter={onenter}
    onmouseleave={onleave}
    {onpointerdown}
    {onpointermove}
    {onpointerup}
    onpointercancel={onpointerup}
  >
    {#if notice}
      <span class="row compact"><span class="label">{notice}</span></span>
    {:else if expanded}
      <!-- Without items, the notes alone are centered. -->
      <ul class="list" class:centered={items.length === 0}>
        {#each items as item (item.id)}
          <li class="row">
            {#if item.dot}
              <span
                class="dot"
                class:pulse={item.tone === 'active'}
                style:background={toneColor(item.tone)}
              ></span>
            {:else if dotted.length > 0}
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
                  {@const Icon = action.icon ? actionIcons[action.icon] : null}
                  <button
                    class="action"
                    class:text={!Icon}
                    title={action.label}
                    aria-label={action.label}
                    onclick={(e) => {
                      // Not an acknowledgement of the whole notch.
                      e.stopPropagation()
                      onaction(item.id, action.id)
                    }}
                  >
                    {#if Icon}<Icon size={13} strokeWidth={2.5} />{:else}{action.label}{/if}
                  </button>
                {/each}
              </span>
            {/if}
          </li>
        {/each}
        <!-- Modules without items still say how they are (ADR-0009). -->
        {#each notes as note, i (i)}
          <li class="row"><span class="label muted note">{note}</span></li>
        {/each}
      </ul>
    {:else if vertical}
      <!-- Thin vertical notch: dots only, details on hover. -->
      <span class="dots column">
        {#each dotted as item (item.id)}
          <span
            class="dot"
            class:pulse={item.tone === 'active'}
            style:background={toneColor(item.tone)}
          ></span>
        {/each}
      </span>
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
</div>

<style>
  .frame {
    position: relative;
    height: 100%;
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
      left 250ms ease,
      top 250ms ease,
      width 250ms ease,
      height 250ms ease,
      border-radius 250ms ease;
  }

  /* Move mode: a grab cursor and a light outline say the notch can be dragged.
     No animation: the shape must follow the cursor at once. */
  .notch.movable {
    cursor: grab;
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.35);
    transition: none;
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
     shorter than its compact shape, which leaves room around a short list. */
  .list {
    list-style: none;
    box-sizing: border-box;
    height: 100%;
    margin: 0;
    padding: 8px 0 10px;
    display: grid;
    align-content: center;
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
