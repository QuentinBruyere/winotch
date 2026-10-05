<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Notch from './lib/Notch.svelte'
  import { newAlerts, soundFor, type Content, type Edge, type Status } from './lib/content'
  import { playSound } from './lib/sound'

  // Notch sizes in logical pixels (DF-0003, DF-0006). The window itself is
  // fixed at the expanded maximum (tauri.conf.json: 380 x 174) and only its
  // clickable area follows the shape; compact shapes must match `compact_shape`
  // in src-tauri/src/notch.rs. The cursor resistance follows the rounded corners.
  const COMPACT_RADIUS = 14
  const EXPANDED_RADIUS = 20
  // Thin and vertical on the left / right edges.
  const compactShape = (edge: Edge) =>
    edge === 'left' || edge === 'right'
      ? { width: 36, height: 120, radius: COMPACT_RADIUS }
      : { width: 300, height: 36, radius: COMPACT_RADIUS }
  const EXPANDED_WIDTH = 380
  const ROW_HEIGHT = 26
  const MAX_ROWS = 6
  const ALERT_MS = 4000
  const ANIMATION_MS = 260

  let content = $state<Content>({ items: [], notes: [] })
  let status = $state<Status>({
    soundEnabled: true,
    edge: 'top',
    movable: false,
    anchor: null,
  })
  let notice = $state<string | null>(null)
  let hovered = $state(false)
  let alerting = $state(false)

  const vertical = $derived(status.edge === 'left' || status.edge === 'right')
  // Opens on hover even without items, to show the modules' notes (ADR-0009).
  // A vertical notch also opens to show notices. In move mode it stays
  // compact, so the shape being dragged does not change under the cursor.
  const expanded = $derived((hovered || alerting) && !notice && !status.movable)
  const rows = $derived(content.items.length || content.notes.length)
  const wideNotice = $derived(notice !== null && vertical)
  // Opening never makes the notch shorter than its compact shape: a vertical
  // notch (120 px tall) would otherwise shrink to one row (44 px) and leave
  // the cursor outside, closing it again as soon as its ends are hovered.
  const shape = $derived(
    expanded || wideNotice
      ? {
          width: EXPANDED_WIDTH,
          height: Math.max(
            compactShape(status.edge).height,
            18 + Math.max(1, Math.min(wideNotice ? 1 : rows, MAX_ROWS)) * ROW_HEIGHT,
          ),
          radius: EXPANDED_RADIUS,
        }
      : compactShape(status.edge),
  )
  const open = $derived(expanded || wideNotice)

  // Dragging in move mode (DF-0006, step 3): the backend reads the cursor
  // position itself, in physical pixels, so the front only says when. One
  // move in flight at a time: extra pointer events are dropped.
  let dragging = false
  let moving = false

  function startDrag(e: PointerEvent) {
    if (!status.movable || e.button !== 0) return
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
    dragging = true
    void invoke('start_drag')
  }

  function drag() {
    if (!dragging || moving) return
    moving = true
    invoke('drag').finally(() => (moving = false))
  }

  function endDrag() {
    if (!dragging) return
    dragging = false
    void invoke('end_drag')
  }

  let alertTimer: ReturnType<typeof setTimeout> | undefined

  function onContent(next: Content) {
    const alerts = newAlerts(content.items, next.items)
    content = next
    if (alerts.length === 0) return

    // One sound only, for the most important alert.
    const kinds = alerts.map((i) => soundFor(i.tone))
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
    if (open) {
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
      listen<Content>('content-changed', (e) => onContent(e.payload)),
      listen<Status>('status-changed', (e) => (status = e.payload)),
      listen<string>('notice', (e) => {
        notice = e.payload
        clearTimeout(noticeTimer)
        noticeTimer = setTimeout(() => (notice = null), 3000)
      }),
    ]
    // Initial state: no sound or expansion for items that were already there.
    invoke<Content>('get_content').then((c) => (content = c))
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
  items={content.items}
  notes={content.notes}
  {notice}
  edge={status.edge}
  {expanded}
  width={shape.width}
  height={shape.height}
  radius={shape.radius}
  movable={status.movable}
  anchor={status.anchor}
  onclick={() => !status.movable && invoke('acknowledge')}
  onpointerdown={startDrag}
  onpointermove={drag}
  onpointerup={endDrag}
  onenter={() => (hovered = true)}
  onleave={() => (hovered = false)}
/>
