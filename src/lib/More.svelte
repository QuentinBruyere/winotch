<script lang="ts">
  import { t } from './i18n.svelte'

  // "+N": items a compact view leaves out (DF-0023), the open notch lists
  // them all.
  let { count }: { count: number } = $props()
</script>

<span class="more" title={t('notch.more', { count })}>+{count}</span>

<style>
  .more {
    flex-shrink: 0;
    font-size: 11px;
    font-weight: 600;
    opacity: 0.55;
    font-variant-numeric: tabular-nums;
  }
</style>
