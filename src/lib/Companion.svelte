<script lang="ts" module>
  // When each item's companion entered its tone, by item: an animation
  // redrawn elsewhere (the notch opened, a pin) goes on where it was, and
  // the arrival is not played again.
  const started = new Map<string, { state: string; at: number }>()
</script>

<script lang="ts">
  import { toneColor, type CompanionSize, type Tone } from './content'
  import {
    center,
    moodAnimation,
    OVERLAP,
    pictures,
    played,
    RATIO,
  } from './companions/companion'
  import type { Mood } from './companions/mood.svelte'
  import { SIZES } from './companions'
  import { companionById } from './companions/custom.svelte'

  // A companion (DF-0020) in place of an item's dot: its animation for the
  // tone, in the tone's color. The frames sit side by side in one SVG that
  // CSS animations slide in steps (no script running, compositor only): the
  // arrival once, then the loop; each distinct picture is drawn once and
  // reused. A single picture without arrival stays still.
  // `item`: the item it stands for; `compact`: in the closed notch or a pin,
  // where it is drawn bigger.
  // `color`: its own color (`--companion-<color>`), worn while working and,
  // darker, at rest; the other states keep theirs, so they stay readable.
  // `dimmed`: darker while working too, as at rest (a color too bright).
  // `mood`: plays that mood's animation instead of the tone's (DF-0024);
  // `tone` still decides the color. `replay`: a new value plays the same
  // mood again from the start (each click petting it).
  let {
    item,
    id,
    size,
    color = null,
    dimmed = false,
    tone,
    mood = null,
    replay = 0,
    compact = false,
  }: {
    item: string
    id: string
    size: CompanionSize
    color?: string | null
    dimmed?: boolean
    tone: Tone
    mood?: Mood | null
    replay?: number
    compact?: boolean
  } = $props()

  // What it plays: a mood, else its state, and how many times it restarted.
  const state = $derived(`${mood ?? tone}:${replay}`)

  const painted = $derived.by(() => {
    const own = color && (tone === 'active' || tone === 'neutral')
    // A palette name, or a `#rrggbb` picked in the settings.
    const base = own
      ? color.startsWith('#')
        ? color
        : `var(--companion-${color})`
      : toneColor(tone)
    const darker = (own && tone === 'neutral') || (dimmed && tone === 'active')
    return darker ? `color-mix(in srgb, ${base} 60%, var(--notch-bg))` : base
  })

  // Ids of the pictures, unique in the page.
  const uid = $props.id()

  // When the item entered this state (tone or mood), and the seconds since.
  const entered = $derived.by(() => {
    const now = performance.now()
    const known = started.get(item)
    if (known?.state !== state) {
      started.set(item, { state, at: now })
      return { at: now, elapsed: 0 }
    }
    return { at: known.at, elapsed: (now - known.at) / 1000 }
  })
  const elapsed = $derived(entered.elapsed)

  const companion = $derived(companionById(id))
  // `#` in its color; an imported companion's own colors (DF-0025).
  const fill = (char: string) =>
    char === '#' ? 'currentColor' : (companion?.palette?.[char] ?? 'currentColor')
  // Scenes in random order drawn when the state started (`played`).
  const animation = $derived(
    companion &&
      played(mood ? moodAnimation(companion, mood) : companion.animations[tone], entered.at),
  )
  const drawn = $derived(animation ? pictures(animation) : { paths: [], order: [] })
  const count = $derived(drawn.order.length)

  // The CSS animations: the arrival, then the loop, both moved back by the
  // time already spent in this tone.
  const playing = $derived.by(() => {
    if (!animation) return ''
    const intro = animation.intro?.length ?? 0
    const loop = animation.frames.length
    const introTime = intro / animation.fps
    const parts: string[] = []
    if (intro > 0) {
      parts.push(`companion-intro ${introTime}s steps(${intro}) ${-elapsed}s 1 forwards`)
    }
    if (loop > 1) {
      parts.push(
        `companion-loop ${loop / animation.fps}s steps(${loop}) ${introTime - elapsed}s infinite`,
      )
    }
    return parts.join(', ')
  })
</script>

{#if companion && animation && count > 0}
  {@const width = companion.width}
  {@const height = companion.height}
  {@const intro = animation.intro?.length ?? 0}
  <span
    class="companion"
    style:--size="{SIZES[size][compact ? 'compact' : 'row']}px"
    style:--shift={(height / 2 - center(companion)) / height}
    style:--count={count}
    style:--ratio={RATIO}
    style:--overlap={OVERLAP}
    style:--intro-end="{(-100 * intro) / count}%"
    style:color={painted}
  >
    {#if count === 1}
      <svg viewBox="0 0 {width} {height}" shape-rendering="crispEdges" aria-hidden="true">
        {#each drawn.paths[0] as part (part.char)}
          <path d={part.d} fill={fill(part.char)} />
        {/each}
      </svg>
    {:else}
      <!-- Keyed on the state: a new one starts its animation from the start. -->
      {#key state}
        <svg
          class="playing"
          viewBox="0 0 {width * count} {height}"
          shape-rendering="crispEdges"
          aria-hidden="true"
          style:animation={playing}
        >
          <defs>
            {#each drawn.paths as picture, k (k)}
              <g id="{uid}-{k}">
                {#each picture as part (part.char)}
                  <path d={part.d} fill={fill(part.char)} />
                {/each}
              </g>
            {/each}
          </defs>
          {#each drawn.order as k, i (i)}
            <use href="#{uid}-{k}" x={i * width} />
          {/each}
        </svg>
      {/key}
    {/if}
  </span>
{/if}

<style>
  /* Moved up or down by whole pixels, so its body is centered. The room on
     its sides, for its scenes, is empty at rest: it overlaps its neighbours
     (another companion, the texts) instead of pushing them away. */
  .companion {
    display: block;
    flex-shrink: 0;
    width: calc(var(--size) * var(--ratio));
    height: var(--size);
    margin-inline: calc(var(--size) * var(--overlap) * -1);
    overflow: hidden;
    translate: 0 round(calc(var(--size) * var(--shift)), 1px);
    transition: color 250ms ease;
  }

  svg {
    display: block;
    width: calc(var(--size) * var(--ratio));
    height: var(--size);
  }

  /* All the frames side by side, slid one frame at a time. */
  .playing {
    width: calc(var(--size) * var(--ratio) * var(--count));
  }

  /* The arrival: from the first frame to the loop's first. */
  @keyframes -global-companion-intro {
    to {
      transform: translateX(var(--intro-end));
    }
  }

  /* The loop: from its first frame to past its last. */
  @keyframes -global-companion-loop {
    from {
      transform: translateX(var(--intro-end));
    }
    to {
      transform: translateX(-100%);
    }
  }
</style>
