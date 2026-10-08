<script lang="ts">
  import Companion from './Companion.svelte'
  import { toneColor, type Item } from './content'

  // An item's dot in its tone's color, or its companion (DF-0020).
  // `compact`: in the closed notch or a pin, where a companion has more room.
  let { item, compact = false }: { item: Item; compact?: boolean } = $props()

</script>

{#if item.companion}
  <Companion
    item={item.id}
    id={item.companion.id}
    size={item.companion.size}
    color={item.companion.color}
    tone={item.tone}
    {compact}
  />
{:else}
  <span class="dot" class:pulse={item.tone === 'active'} style:background={toneColor(item.tone)}
  ></span>
{/if}
