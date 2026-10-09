<script lang="ts">
  import Companion from './Companion.svelte'
  import { toneColor, type Item } from './content'
  import { moodTone, notchMood, type Mood } from './companions/mood.svelte'

  // An item's dot in its tone's color, or its companion (DF-0020).
  // `compact`: in the closed notch or a pin, where a companion has more room.
  let { item, compact = false }: { item: Item; compact?: boolean } = $props()

  // A reactive companion (the Companion module, DF-0024): its mood follows
  // the notch, the mouse wakes it up, a click pets it for a moment.
  const PETTED_MS = 2500
  let hovered = $state(false)
  let petted = $state(false)
  // Each click plays the petting again, from the start.
  let pets = $state(0)
  let petTimer: ReturnType<typeof setTimeout> | undefined

  const mood = $derived.by((): Mood | null => {
    if (!item.companion?.reactive) return null
    const fromNotch = notchMood()
    if (fromNotch === 'alarm') return 'alarm'
    if (petted) return 'petted'
    if (hovered) return 'watch'
    return fromNotch
  })

  function pet(e: MouseEvent) {
    // Not a click on the notch or the pin.
    e.stopPropagation()
    petted = true
    pets += 1
    clearTimeout(petTimer)
    petTimer = setTimeout(() => (petted = false), PETTED_MS)
  }
</script>

{#if item.companion && mood}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <span
    class="pet"
    onmouseenter={() => (hovered = true)}
    onmouseleave={() => (hovered = false)}
    onclick={pet}
  >
    <Companion
      item={item.id}
      id={item.companion.id}
      size={item.companion.size}
      color={item.companion.color}
      dimmed={item.companion.dimmed}
      tone={moodTone(mood)}
      {mood}
      replay={mood === 'petted' ? pets : 0}
      {compact}
    />
  </span>
{:else if item.companion}
  <Companion
    item={item.id}
    id={item.companion.id}
    size={item.companion.size}
    color={item.companion.color}
    dimmed={item.companion.dimmed}
    tone={item.tone}
    {compact}
  />
{:else}
  <span class="dot" class:pulse={item.tone === 'active'} style:background={toneColor(item.tone)}
  ></span>
{/if}

<style>
  .pet {
    display: contents;
    cursor: pointer;
  }
</style>
