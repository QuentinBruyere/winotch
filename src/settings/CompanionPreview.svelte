<script lang="ts">
  import Companion from '../lib/Companion.svelte'
  import type { Animation, Companion as Drawing } from '../lib/companions/companion'
  import { companionById } from '../lib/companions/custom.svelte'
  import { moodTone, type Mood } from '../lib/companions/mood.svelte'
  import type { Tone } from '../lib/content'
  import { t, type Key } from '../lib/i18n.svelte'

  // A companion played big, one animation at a time: each state, each mood
  // it has its own, and each scene of them on its own (DF-0025), to see
  // everything it does without waiting for it to happen. A panel that
  // opens and closes (`open`), closed at first.
  let {
    id,
    name,
    open = $bindable(false),
  }: { id: string; name: string; open?: boolean } = $props()

  type Entry = { label: string; animation: Animation; tone: Tone }

  const TONES: Tone[] = ['neutral', 'active', 'attention', 'question', 'success', 'error']
  const MOODS: Mood[] = ['idle', 'dance', 'run', 'watch', 'petted', 'stretch', 'sleep', 'alarm']

  function entries(companion: Drawing): Entry[] {
    const list: Entry[] = []
    const add = (label: string, animation: Animation, tone: Tone) => {
      // The animation as it plays, then each of its scenes alone.
      list.push({ label, animation: { ...animation, scenes: undefined }, tone })
      animation.scenes?.forEach((scene, i) =>
        list.push({
          label: t('settings.companions.scene', { name: label, n: i + 1 }),
          animation: { fps: animation.fps, frames: scene },
          tone,
        }),
      )
    }
    for (const tone of TONES) {
      add(t(`settings.companions.anim.${tone}` as Key), companion.animations[tone], tone)
    }
    for (const mood of MOODS) {
      const own = companion.moods[mood]
      if (own) add(t(`settings.companions.anim.${mood}` as Key), own, moodTone(mood))
    }
    return list
  }

  const companion = $derived(companionById(id))
  const list = $derived(companion ? entries(companion) : [])
  let chosen = $state(1)
  // Each click plays the animation again from the start.
  let replay = $state(0)
  const entry = $derived(list[Math.min(chosen, list.length - 1)])

  // Another companion: back to its working animation.
  $effect(() => {
    void id
    chosen = 1
  })

  function play(i: number) {
    chosen = i
    replay += 1
  }
</script>

<details class="companion-animations separated" bind:open>
  <summary>{t('settings.companions.preview', { name })}</summary>
  {#if open && entry}
    <div class="companion-stage">
      <Companion
        item="settings-preview"
        {id}
        size="large"
        tone={entry.tone}
        animation={entry.animation}
        {replay}
        compact
      />
    </div>
    <div class="animation-choices">
      {#each list as item, i (i)}
        <button class:selected={i === chosen} onclick={() => play(i)}>{item.label}</button>
      {/each}
    </div>
  {/if}
</details>
