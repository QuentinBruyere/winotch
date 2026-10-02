<script lang="ts">
  import { stateColor, stateLabels, type SessionState } from './session'

  let { state }: { state: SessionState } = $props()
</script>

<div class="notch">
  <span
    class="dot"
    class:pulse={state === 'working'}
    style:background={stateColor(state)}
  ></span>
  <span class="label">{stateLabels[state]}</span>
</div>

<style>
  .notch {
    box-sizing: border-box;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 0 16px 2px;
    background: var(--notch-bg);
    border-radius: 0 0 var(--notch-radius) var(--notch-radius);
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

  .label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
</style>
