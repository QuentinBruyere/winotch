<script lang="ts">
  import type { LucideProps } from '@lucide/svelte'
  import type { Component } from 'svelte'
  import type { Item } from './content'

  // A module's icon (or an item's, e.g. the volume's level). When `item`
  // makes its icon a button (`Item::icon_action`, e.g. mute), clicking it
  // runs that action, and only that: not the click on the notch or pin.
  let {
    icon: Icon,
    item = null,
    onaction,
  }: {
    icon: Component<LucideProps>
    item?: Item | null
    onaction?: (itemId: string, actionId: string) => void
  } = $props()

  const action = $derived(onaction ? (item?.iconAction ?? null) : null)
</script>

{#if item && action && onaction}
  <button
    class="module-icon icon-button"
    title={action.label}
    aria-label={action.label}
    onclick={(e) => {
      e.stopPropagation()
      onaction(item.id, action.id)
    }}
  >
    <Icon size={14} strokeWidth={2.25} />
  </button>
{:else}
  <span class="module-icon"><Icon size={14} strokeWidth={2.25} /></span>
{/if}

<style>
  /* Looks like the icon it replaces; lights up under the cursor. */
  .icon-button {
    all: unset;
    display: grid;
    place-items: center;
    width: 24px;
    margin-inline: -5px;
    border-radius: 6px;
    cursor: pointer;
    transition:
      background-color 150ms ease,
      opacity 150ms ease;
  }

  .icon-button:hover {
    opacity: 1;
    background: var(--notch-button);
  }
</style>
