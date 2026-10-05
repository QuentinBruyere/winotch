<script lang="ts">
  import {
    mostUrgent,
    projectName,
    stateColor,
    stateLabels,
    type Edge,
    type Session,
    type Status,
  } from './session'

  let {
    sessions,
    status,
    notice,
    edge,
    expanded,
    width,
    height,
    radius,
    movable,
    anchor,
    onclick,
    onenter,
    onleave,
    onpointerdown,
    onpointermove,
    onpointerup,
  }: {
    sessions: Session[]
    status: Status
    notice: string | null
    edge: Edge
    expanded: boolean
    width: number
    height: number
    radius: number
    movable: boolean
    anchor: number | null
    onclick: () => void
    onenter: () => void
    onleave: () => void
    onpointerdown: (e: PointerEvent) => void
    onpointermove: () => void
    onpointerup: () => void
  } = $props()

  const top = $derived(mostUrgent(sessions))
  const vertical = $derived(edge === 'left' || edge === 'right')

  // Shown when there is nothing to report about Claude Code itself.
  const fallback = $derived(
    status.serverError ??
      (status.hooksInstalled ? 'Aucune session' : 'Claude Code non connecté'),
  )

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

  // Rounded corners on the inner side only, the attached side stays square
  // (DF-0006). Order: top-left, top-right, bottom-right, bottom-left.
  const corners = $derived(
    {
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
  <button
    class="notch"
    class:vertical={vertical && !expanded && !notice}
    class:movable
    style={position}
    style:width="{width}px"
    style:height="{height}px"
    style:border-radius={corners}
    {onclick}
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
      <ul class="list">
        {#each sessions as session (session.id)}
          <li class="row">
            <span
              class="dot"
              class:pulse={session.state === 'working'}
              style:background={stateColor(session.state)}
            ></span>
            <span class="project">{projectName(session)}</span>
            <span class="label">
              {stateLabels[session.state]}{session.tool ? ` · ${session.tool}` : ''}
            </span>
          </li>
        {:else}
          <li class="row"><span class="label muted">{fallback}</span></li>
        {/each}
      </ul>
    {:else if vertical}
      <!-- Thin vertical notch: session dots only, details on hover. -->
      <span class="dots column">
        {#each sessions as session (session.id)}
          <span
            class="dot"
            class:pulse={session.state === 'working'}
            style:background={stateColor(session.state)}
          ></span>
        {:else}
          <span class="dot idle" title={fallback}></span>
        {/each}
      </span>
    {:else if top}
      <span class="row compact">
        <span class="dots">
          {#each sessions as session (session.id)}
            <span
              class="dot"
              class:pulse={session.state === 'working'}
              style:background={stateColor(session.state)}
            ></span>
          {/each}
        </span>
        <span class="label">{stateLabels[top.state]}</span>
        <span class="muted">{projectName(top)}</span>
      </span>
    {:else}
      <span class="row compact"><span class="label muted">{fallback}</span></span>
    {/if}
  </button>
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

  .list {
    list-style: none;
    margin: 0;
    padding: 8px 0 10px;
    display: grid;
    gap: 2px;
  }

  .list .row {
    height: 24px;
  }

  .list .project {
    flex: 0 1 auto;
    font-weight: 600;
  }

  .list .label {
    margin-left: auto;
    opacity: 0.75;
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

  .dot.idle {
    background: var(--state-idle);
  }

  .pulse {
    animation: pulse 1.2s ease-in-out infinite;
  }

  .label,
  .project,
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
