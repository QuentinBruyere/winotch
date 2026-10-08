<script lang="ts">
  import type { LucideProps } from '@lucide/svelte'
  import type { Component } from 'svelte'

  // An item's picture (an album cover, DF-0017). Without one (empty `src`),
  // or if it cannot be shown, the module's icon stands in.
  let {
    src,
    icon: Icon,
    dimmed = false,
  }: { src: string; icon: Component<LucideProps>; dimmed?: boolean } = $props()

  // The picture that failed to load: a new one gets its chance.
  let failed = $state<string | null>(null)
</script>

{#if src && failed !== src}
  <img class="cover" class:dimmed {src} alt="" onerror={() => (failed = src)} />
{:else}
  <span class="cover placeholder" class:dimmed><Icon size={12} strokeWidth={2.25} /></span>
{/if}
