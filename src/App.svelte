<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Notch from './lib/Notch.svelte'
  import type { Session, Status } from './lib/session'

  let sessions = $state<Session[]>([])
  let status = $state<Status>({ serverError: null, hooksInstalled: false })
  let notice = $state<string | null>(null)

  $effect(() => {
    let noticeTimer: ReturnType<typeof setTimeout> | undefined
    const unlisten = [
      listen<Session[]>('sessions-changed', (e) => (sessions = e.payload)),
      listen<Status>('status-changed', (e) => (status = e.payload)),
      listen<string>('notice', (e) => {
        notice = e.payload
        clearTimeout(noticeTimer)
        noticeTimer = setTimeout(() => (notice = null), 3000)
      }),
    ]
    invoke<Session[]>('get_sessions').then((s) => (sessions = s))
    invoke<Status>('get_status').then((s) => (status = s))

    return () => {
      clearTimeout(noticeTimer)
      unlisten.forEach((p) => p.then((off) => off()))
    }
  })
</script>

<Notch {sessions} {status} {notice} onclick={() => invoke('acknowledge')} />
