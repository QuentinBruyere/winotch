<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Notch from './lib/Notch.svelte'
  import {
    allItems,
    moduleOf,
    newAlerts,
    soundFor,
    speedMs,
    type Card,
    type Content,
    type Edge,
    type Section,
    type Status,
    type Style,
  } from './lib/content'
  import { playSound } from './lib/sound'
  import type { ModuleUis } from './modules'

  let { moduleUis }: { moduleUis: ModuleUis } = $props()

  // Notch sizes in logical pixels (DF-0003, DF-0006). The window itself is
  // fixed at the expanded maximum (tauri.conf.json: 380 x MAX_HEIGHT) and only its
  // clickable area follows the shape; compact shapes must match `compact_shape`
  // in src-tauri/src/notch.rs. The cursor resistance follows the rounded corners.
  // A pill is fully rounded: its radius is half its thickness.
  const compactRadius = (style: Style) => (style === 'pill' ? 18 : 14)
  const EXPANDED_RADIUS = 20
  // Thin and vertical on the left / right edges.
  const compactShape = (edge: Edge, style: Style) =>
    edge === 'left' || edge === 'right'
      ? { width: 36, height: 120, radius: compactRadius(style) }
      : { width: 300, height: 36, radius: compactRadius(style) }
  const EXPANDED_WIDTH = 380
  const ROW_HEIGHT = 26
  // Added by the line between two modules (`.section + .section` in Notch.svelte).
  const SEPARATOR_HEIGHT = 15
  // The window height (tauri.conf.json): past it, the open notch scrolls.
  const MAX_HEIGHT = 300
  // Separate layout (DF-0011): space between two cards, which must match
  // `CARD_GAP` in src-tauri/src/notch.rs, and the corners of the extra cards.
  const CARD_GAP = 8
  const CARD_RADIUS = 16
  // A card never gets shorter than one row when the stack must shrink.
  const MIN_CARD_HEIGHT = 18 + ROW_HEIGHT
  const ALERT_MS = 4000

  let content = $state<Content>({ sections: [], note: null })
  let status = $state<Status>({
    soundEnabled: true,
    edge: 'top',
    style: 'notch',
    movable: false,
    layout: 'joined',
    speed: 'normal',
    anchor: null,
  })
  let notice = $state<string | null>(null)
  let hovered = $state(false)
  let alerting = $state(false)
  // Modules whose alert opened the notch: only their part shows (DF-0011).
  let alertModules = $state<Set<string>>(new Set())

  const vertical = $derived(status.edge === 'left' || status.edge === 'right')
  // The opening / closing animation, set in the settings.
  const duration = $derived(speedMs[status.speed])
  // Opens on hover even without items, to show the modules' notes (ADR-0009).
  // A vertical notch also opens to show notices. In move mode it stays
  // compact, so the shape being dragged does not change under the cursor.
  const expanded = $derived((hovered || alerting) && !notice && !status.movable)
  // Hovered, every module shows; opened by an alert, only the alerting ones.
  const visibleSections = $derived.by(() => {
    if (hovered) return content.sections
    const alerted = content.sections.filter((s) => alertModules.has(s.module))
    return alerted.length > 0 ? alerted : content.sections
  })
  const wideNotice = $derived(notice !== null && vertical)
  // Separate layout: one card per module, once there are several to show.
  const separate = $derived(status.layout === 'separate' && visibleSections.length > 1)

  // Height for some modules: a module without items still takes a row, for
  // its note; a line between two modules.
  function heightOf(sections: Section[]): number {
    const rows = Math.max(
      1,
      sections.reduce((n, s) => n + Math.max(1, s.items.length), 0),
    )
    return 18 + rows * ROW_HEIGHT + Math.max(0, sections.length - 1) * SEPARATOR_HEIGHT
  }

  // Shrinks the tallest cards until the stack fits in the window; their
  // content then scrolls.
  function fit(heights: number[], max: number): number[] {
    let excess = heights.reduce((a, b) => a + b, 0) - max
    while (excess > 0) {
      const i = heights.indexOf(Math.max(...heights))
      const cut = Math.min(excess, heights[i] - MIN_CARD_HEIGHT)
      if (cut <= 0) break
      heights[i] -= cut
      excess -= cut
    }
    return heights
  }

  const cards = $derived.by<Card[]>(() => {
    const compact = compactShape(status.edge, status.style)
    if (!expanded && !wideNotice) return [{ ...compact, sections: [] }]
    const groups = separate ? visibleSections.map((s) => [s]) : [visibleSections]
    const heights = groups.map((g) => (wideNotice ? 18 + ROW_HEIGHT : heightOf(g)))
    // Opening never makes the notch shorter than its compact shape: a
    // vertical notch (120 px tall) would otherwise shrink to one row (44 px)
    // and leave the cursor outside, closing it as soon as its ends are hovered.
    heights[0] = Math.max(compact.height, heights[0])
    fit(heights, MAX_HEIGHT - CARD_GAP * (groups.length - 1))
    return groups.map((sections, i) => ({
      width: EXPANDED_WIDTH,
      height: heights[i],
      radius: i === 0 ? EXPANDED_RADIUS : CARD_RADIUS,
      sections: wideNotice ? [] : sections,
    }))
  })
  const open = $derived(expanded || wideNotice)
  // What the window region needs, as a string: the cards are rebuilt whenever
  // a module redraws (every second for a running stopwatch), the region only
  // changes with this.
  const hitArea = $derived(
    JSON.stringify(cards.map(({ width, height, radius }) => ({ width, height, radius }))),
  )


  // Open, the notch closes only once the cursor is past a safety margin
  // around it, watched by the backend (the window ends at the cards' edge).
  let watchingLeave = false

  function enter() {
    if (watchingLeave) void invoke('cancel_leave')
    watchingLeave = false
    hovered = true
  }

  function leave() {
    if (expanded) {
      watchingLeave = true
      void invoke('watch_leave')
    } else {
      hovered = false
    }
  }

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
    const alerts = newAlerts(allItems(content), allItems(next))
    content = next
    if (alerts.length === 0) return

    // One sound only, for the most important alert.
    const kinds = alerts.map((i) => soundFor(i.tone))
    const kind = (['attention', 'error', 'done'] as const).find((k) => kinds.includes(k))
    if (kind && status.soundEnabled) playSound(kind)

    // An alert arriving while another shows adds its module to the open notch.
    alertModules = new Set([...(alerting ? alertModules : []), ...alerts.map(moduleOf)])
    alerting = true
    clearTimeout(alertTimer)
    alertTimer = setTimeout(() => (alerting = false), ALERT_MS)
  }

  // The clickable area grows before the shape animates open, and shrinks only
  // once the shape has finished closing, so the animation is never clipped.
  let resizeTimer: ReturnType<typeof setTimeout> | undefined
  $effect(() => {
    const cards = JSON.parse(hitArea)
    clearTimeout(resizeTimer)
    if (open) {
      void invoke('set_hit_area', { cards })
    } else {
      resizeTimer = setTimeout(() => void invoke('set_hit_area', { cards }), duration + 10)
    }
  })

  $effect(() => {
    let noticeTimer: ReturnType<typeof setTimeout> | undefined
    const unlisten = [
      listen<Content>('content-changed', (e) => onContent(e.payload)),
      listen<Status>('status-changed', (e) => (status = e.payload)),
      // Past the safety margin of the open notch (see `leave`).
      listen('pointer-left', () => (hovered = false)),
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
  {cards}
  foldedWidth={compactShape(status.edge, status.style).width}
  {duration}
  gap={CARD_GAP}
  allSections={content.sections}
  note={content.note}
  {moduleUis}
  {notice}
  edge={status.edge}
  style={status.style}
  {expanded}
  movable={status.movable}
  anchor={status.anchor}
  onclick={() => !status.movable && invoke('acknowledge')}
  onaction={(item, action) => void invoke('item_action', { item, action })}
  onpointerdown={startDrag}
  onpointermove={drag}
  onpointerup={endDrag}
  onenter={enter}
  onleave={leave}
/>
