<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Notch from './lib/Notch.svelte'
  import { newAlerts, soundFor, type Session, type Status } from './lib/session'
  import { playSound } from './lib/sound'

  // Notch sizes in logical pixels (DF-0003). The window itself is fixed at the
  // expanded maximum (tauri.conf.json: 380 x 174) and only its clickable area
  // follows the shape; COMPACT must match HIT_AREA in src-tauri/src/notch.rs.
  // Radii must match the CSS of src/lib/Notch.svelte (--notch-radius, .expanded):
  // the cursor resistance follows the rounded corners.
  const COMPACT = { width: 300, height: 36, radius: 14 }
  const EXPANDED_RADIUS = 20
  const EXPANDED_WIDTH = 380
  const ROW_HEIGHT = 26
  const MAX_ROWS = 6
  const ALERT_MS = 4000
  const ANIMATION_MS = 260

  let sessions = $state<Session[]>([])
  let status = $state<Status>({
    serverError: null,
    hooksInstalled: false,
    soundEnabled: true,
  })
  let notice = $state<string | null>(null)
  let hovered = $state(false)
  let alerting = $state(false)

  const expanded = $derived((hovered || alerting) && sessions.length > 0 && !notice)
  const shape = $derived(
    expanded
      ? {
          width: EXPANDED_WIDTH,
          height: 18 + Math.min(sessions.length, MAX_ROWS) * ROW_HEIGHT,
          radius: EXPANDED_RADIUS,
        }
      : COMPACT,
  )

  let alertTimer: ReturnType<typeof setTimeout> | undefined

  function onSessions(next: Session[]) {
    const alerts = newAlerts(sessions, next)
    sessions = next
    if (alerts.length === 0) return

    // One sound only, for the most important alert.
    const kinds = alerts.map((s) => soundFor(s.state))
    const kind = (['attention', 'error', 'done'] as const).find((k) => kinds.includes(k))
    if (kind && status.soundEnabled) playSound(kind)

    alerting = true
    clearTimeout(alertTimer)
    alertTimer = setTimeout(() => (alerting = false), ALERT_MS)
  }

  // The clickable area grows before the shape animates open, and shrinks only
  // once the shape has finished closing, so the animation is never clipped.
  let resizeTimer: ReturnType<typeof setTimeout> | undefined
  $effect(() => {
    const { width, height, radius } = shape
    clearTimeout(resizeTimer)
    if (expanded) {
      void invoke('set_hit_area', { width, height, radius })
    } else {
      resizeTimer = setTimeout(
        () => void invoke('set_hit_area', { width, height, radius }),
        ANIMATION_MS,
      )
    }
  })

  $effect(() => {
    let noticeTimer: ReturnType<typeof setTimeout> | undefined
    const unlisten = [
      listen<Session[]>('sessions-changed', (e) => onSessions(e.payload)),
      listen<Status>('status-changed', (e) => (status = e.payload)),
      listen<string>('notice', (e) => {
        notice = e.payload
        clearTimeout(noticeTimer)
        noticeTimer = setTimeout(() => (notice = null), 3000)
      }),
    ]
    // Initial state: no sound or expansion for sessions that were already there.
    invoke<Session[]>('get_sessions').then((s) => (sessions = s))
    invoke<Status>('get_status').then((s) => (status = s))

    return () => {
      clearTimeout(noticeTimer)
      clearTimeout(alertTimer)
      clearTimeout(resizeTimer)
      unlisten.forEach((p) => p.then((off) => off()))
    }
  })
</script>

<Notch
  {sessions}
  {status}
  {notice}
  {expanded}
  width={shape.width}
  height={shape.height}
  onclick={() => invoke('acknowledge')}
  onenter={() => (hovered = true)}
  onleave={() => (hovered = false)}
/>
