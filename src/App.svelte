<script lang="ts">
  import Notch from './lib/Notch.svelte'
  import type { SessionState } from './lib/session'

  // Prototype only: cycles through every state until the Claude Code
  // integration (milestone 2) feeds real events.
  const demo: SessionState[] = [
    'idle',
    'working',
    'needs_permission',
    'working',
    'done',
    'waiting_input',
    'error',
  ]
  let index = $state(0)

  $effect(() => {
    const timer = setInterval(() => (index = (index + 1) % demo.length), 2500)
    return () => clearInterval(timer)
  })
</script>

<Notch state={demo[index]} />
