<script lang="ts">
  import {
    mostUrgent,
    projectName,
    stateColor,
    stateLabels,
    type Session,
    type Status,
  } from './session'

  let {
    sessions,
    status,
    notice,
    expanded,
    width,
    height,
    onclick,
    onenter,
    onleave,
  }: {
    sessions: Session[]
    status: Status
    notice: string | null
    expanded: boolean
    width: number
    height: number
    onclick: () => void
    onenter: () => void
    onleave: () => void
  } = $props()

  const top = $derived(mostUrgent(sessions))

  // Shown when there is nothing to report about Claude Code itself.
  const fallback = $derived(
    status.serverError ??
      (status.hooksInstalled ? 'Aucune session' : 'Claude Code non connecté'),
  )
</script>

<button
  class="notch"
  class:expanded
  style:width="{width}px"
  style:height="{height}px"
  {onclick}
  onmouseenter={onenter}
  onmouseleave={onleave}
>
  {#if notice}
    <span class="row compact"><span class="label">{notice}</span></span>
  {:else if expanded && sessions.length > 0}
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
      {/each}
    </ul>
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

<style>
  .notch {
    all: unset;
    box-sizing: border-box;
    display: block;
    margin: 0 auto;
    overflow: hidden;
    padding: 0 16px;
    background: var(--notch-bg);
    border-radius: 0 0 var(--notch-radius) var(--notch-radius);
    transition:
      width 250ms ease,
      height 250ms ease,
      border-radius 250ms ease;
  }

  .notch.expanded {
    border-radius: 0 0 20px 20px;
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
