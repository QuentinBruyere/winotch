<script lang="ts">
  import {
    describe,
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
    onclick,
  }: {
    sessions: Session[]
    status: Status
    notice: string | null
    onclick: () => void
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
  title={sessions.map(describe).join('\n') || fallback}
  {onclick}
>
  {#if notice}
    <span class="label">{notice}</span>
  {:else if top}
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
    <span class="project">{projectName(top)}</span>
  {:else}
    <span class="label muted">{fallback}</span>
  {/if}
</button>

<style>
  .notch {
    all: unset;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 0 16px 2px;
    background: var(--notch-bg);
    border-radius: 0 0 var(--notch-radius) var(--notch-radius);
  }

  .dots {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    transition: background-color 250ms ease;
  }

  .pulse {
    animation: pulse 1.2s ease-in-out infinite;
  }

  .label,
  .project {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .project,
  .muted {
    opacity: 0.55;
  }

  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
</style>
