<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import Notch from './lib/Notch.svelte'
  import Pin from './lib/Pin.svelte'
  import { setLanguage } from './lib/i18n.svelte'
  import { applyAppearance } from './lib/theme'
  import {
    allItems,
    around,
    corners,
    moduleOf,
    newAlerts,
    ringing,
    soundFor,
    speedMs,
    type Card,
    type Content,
    type Edge,
    type Rect,
    type Section,
    type Shape,
    type Status,
    type Style,
  } from './lib/content'
  import { playSound } from './lib/sound'
  import type { ModuleUis } from './modules'

  let { moduleUis }: { moduleUis: ModuleUis } = $props()

  // Notch sizes in logical pixels (DF-0003, DF-0006). The window spans the
  // whole edge (`placement::window_size`) and only its clickable area
  // follows the shapes, computed here and drawn from the same numbers; the
  // compact shapes must match `compact_shape` in src-tauri/src/notch.rs (used
  // until the first report). The cursor resistance follows the rounded corners.
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
  // Added by the line between two modules (`.section + .section` in Sections.svelte).
  const SEPARATOR_HEIGHT = 15
  // Depth of the window on the top / bottom edges (`WINDOW_DEPTH` in
  // src-tauri/src/notch.rs): past it, the open notch scrolls.
  const MAX_HEIGHT = 300
  // Separate layout (DF-0011): space between two cards, corners of the
  // extra cards.
  const CARD_GAP = 8
  const CARD_RADIUS = 16
  // A card never gets shorter than one row when the stack must shrink.
  const MIN_CARD_HEIGHT = 18 + ROW_HEIGHT
  // Pins (DF-0012): space next to the notch and between pins, smallest
  // closed width, open width, length of a thin pin on the left / right edges.
  const PIN_GAP = 6
  const PIN_MIN_WIDTH = 44
  const PIN_OPEN_WIDTH = 280
  const PIN_VERTICAL_LENGTH = 56
  const ALERT_MS = 4000
  // How long the closed notch shows a module that asked for it (DF-0013).
  const SPOTLIGHT_MS = 1500
  // A ringing module's alarm (DF-0009): every few seconds, for a minute at
  // most; its shape keeps pulsing until it is acknowledged.
  const ALARM_EVERY_MS = 2000
  const ALARM_FOR_MS = 60_000

  let content = $state<Content>({ sections: [], note: null })
  let status = $state<Status>({
    soundEnabled: true,
    edge: 'top',
    style: 'notch',
    movable: false,
    layout: 'joined',
    speed: 'normal',
    appearance: 'system',
    language: 'en',
    anchor: null,
  })
  $effect(() => applyAppearance(status.appearance))
  $effect(() => setLanguage(status.language))
  let notice = $state<string | null>(null)
  let hovered = $state(false)
  let alerting = $state(false)
  // Modules whose alert opened the notch: only their part shows (DF-0011).
  let alertModules = $state<Set<string>>(new Set())
  // The pin under the cursor, and the pins an alert opened (DF-0012).
  let pinHover = $state<string | null>(null)
  // A module that just changed through the user elsewhere (the volume keys):
  // the closed notch, or its pin, shows its value for a moment.
  let spotlight = $state<string | null>(null)
  let pinAlerting = $state(false)
  let pinAlerts = $state<Set<string>>(new Set())
  // Closed width each pin needs, measured by Pin.svelte.
  let pinWidths = $state<Record<string, number>>({})
  let windowWidth = $state(0)
  let windowHeight = $state(0)

  const vertical = $derived(status.edge === 'left' || status.edge === 'right')
  const attached = $derived(status.style === 'notch')
  // The opening / closing animation, set in the settings.
  const duration = $derived(speedMs[status.speed])
  // Pinned modules live in their own mini-notch, not in the notch (DF-0012).
  const notchSections = $derived(content.sections.filter((s) => !s.pin))
  // What the closed notch chooses from: the spotlighted module alone.
  const compactSections = $derived.by(() => {
    const shown = spotlight && notchSections.find((s) => s.module === spotlight)
    return shown ? [shown] : notchSections
  })
  // Opens on hover even without items, to show the modules' notes (ADR-0009).
  // A vertical notch also opens to show notices. In move mode it stays
  // compact, so the shape being dragged does not change under the cursor.
  const expanded = $derived((hovered || alerting) && !notice && !status.movable)
  // Hovered, every module shows; opened by an alert, only the alerting ones.
  const visibleSections = $derived.by(() => {
    if (hovered) return notchSections
    const alerted = notchSections.filter((s) => alertModules.has(s.module))
    return alerted.length > 0 ? alerted : notchSections
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

  // --- Geometry: every shape in the window, drawn and made clickable ---

  // The cards are stacked from the attached side: towards the inside of the
  // screen on the top and bottom edges, down along the edge on the left and
  // right ones. The stack is centered on the anchor along the edge, then kept
  // inside the window: at an end of the edge the notch opens towards the
  // other end.
  const stackWidth = $derived(Math.max(...cards.map((c) => c.width)))
  const stackHeight = $derived(
    cards.reduce((sum, c) => sum + c.height, 0) + CARD_GAP * (cards.length - 1),
  )
  const length = $derived(vertical ? windowHeight : windowWidth)
  const baseStart = $derived.by(() => {
    const size = vertical ? stackHeight : stackWidth
    const center = status.anchor ?? length / 2
    return Math.min(Math.max(center - size / 2, 0), Math.max(length - size, 0))
  })
  function cardRectsAt(start: number): Rect[] {
    let offset = 0
    return cards.map((c) => {
      const at = offset
      offset += c.height + CARD_GAP
      const centered = start + (stackWidth - c.width) / 2
      const corner = {
        top: { x: centered, y: at },
        bottom: { x: centered, y: windowHeight - at - c.height },
        left: { x: 0, y: start + at },
        right: { x: windowWidth - c.width, y: start + at },
      }[status.edge]
      return { ...corner, width: c.width, height: c.height }
    })
  }
  // Pins (DF-0012): next to the notch, the first in display order closest to
  // it; above and below it on the left / right edges. A pin opens outwards
  // (away from the notch) and towards the inside of the screen.
  const pinned = $derived(content.sections.filter((s) => s.pin && s.items.length > 0))
  function pinOpen(module: string): boolean {
    return (
      !status.movable &&
      !notice &&
      (pinHover === module || (pinAlerting && pinAlerts.has(module)))
    )
  }
  function pinsAt(notch: Rect) {
    const compact = compactShape(status.edge, status.style)
    const placed: { section: Section; rect: Rect; open: boolean; radius: number }[] = []
    for (const side of ['right', 'left'] as const) {
      const after = side === 'right'
      let cursor = vertical
        ? after
          ? notch.y + notch.height + PIN_GAP
          : notch.y - PIN_GAP
        : after
          ? notch.x + notch.width + PIN_GAP
          : notch.x - PIN_GAP
      for (const section of pinned.filter((s) => s.pin === side)) {
        const isOpen = pinOpen(section.module)
        const closedLength = vertical
          ? PIN_VERTICAL_LENGTH
          : Math.max(PIN_MIN_WIDTH, pinWidths[section.module] ?? PIN_MIN_WIDTH)
        const thickness = vertical ? compact.width : compact.height
        const width = isOpen ? PIN_OPEN_WIDTH : vertical ? thickness : closedLength
        const height = isOpen
          ? Math.max(vertical ? closedLength : thickness, heightOf([section]))
          : vertical
            ? closedLength
            : thickness
        const along = vertical ? height : width
        const at = after ? cursor : cursor - along
        cursor = after ? cursor + along + PIN_GAP : at - PIN_GAP
        const rect = vertical
          ? { x: status.edge === 'left' ? 0 : windowWidth - width, y: at, width, height }
          : { x: at, y: status.edge === 'top' ? 0 : windowHeight - height, width, height }
        placed.push({
          section,
          rect,
          open: isOpen,
          radius: isOpen ? CARD_RADIUS : compact.radius,
        })
      }
    }
    return placed
  }

  // The window spans the whole edge (`placement::window_size`). When the pins
  // do not fit on a side (the notch pushed to an end), the whole group slides
  // inwards just enough: nothing is ever cut.
  const start = $derived.by(() => {
    const rects = cardRectsAt(baseStart)
    const all = [...rects, ...pinsAt(rects[0]).map((p) => p.rect)]
    const begin = Math.min(...all.map((r) => (vertical ? r.y : r.x)))
    const end = Math.max(...all.map((r) => (vertical ? r.y + r.height : r.x + r.width)))
    if (begin < 0 || end - begin > length) return baseStart - begin
    if (end > length) return baseStart - (end - length)
    return baseStart
  })
  const cardRects = $derived(cardRectsAt(start))
  const pins = $derived(pinsAt(cardRects[0]))

  // The gaps between cards belong to the notch: hovering them keeps it open.
  const gapRects = $derived(
    cardRects.slice(1).map((b, i) => {
      const a = cardRects[i]
      const [upper, lower] = a.y < b.y ? [a, b] : [b, a]
      const x = Math.min(a.x, b.x)
      const y = upper.y + upper.height
      return {
        x,
        y,
        width: Math.max(a.x + a.width, b.x + b.width) - x,
        height: lower.y - y,
      }
    }),
  )


  // The window region: the notch first (the cursor resistance guards it),
  // then the other cards, the gaps between them and the pins. Each has a key,
  // to follow it while it moves.
  const shapes = $derived.by<(Shape & { key: string })[]>(() => [
    { key: 'notch', ...cardRects[0], radius: cards[0].radius, attached },
    ...cardRects.slice(1).map((r, i) => ({
      key: `card:${i + 1}`,
      ...r,
      radius: cards[i + 1].radius,
      attached: false,
    })),
    ...gapRects.map((r, i) => ({ key: `gap:${i}`, ...r, radius: 0, attached: false })),
    ...pins.map((p) => ({ key: `pin:${p.section.module}`, ...p.rect, radius: p.radius, attached })),
  ])
  // As a string: the shapes are rebuilt whenever a module redraws (every
  // second for a running stopwatch), the region only changes with this.
  const hitArea = $derived(
    windowWidth > 0
      ? JSON.stringify(
          shapes.map((s) => ({
            ...s,
            x: Math.round(s.x * 100) / 100,
            y: Math.round(s.y * 100) / 100,
          })),
        )
      : '',
  )

  // The notch and its cards, gaps included: its safety margin goes around this.
  const notchArea = $derived.by<Rect>(() => {
    const rects = [...cardRects, ...gapRects]
    const x = Math.min(...rects.map((r) => r.x))
    const y = Math.min(...rects.map((r) => r.y))
    return {
      x,
      y,
      width: Math.max(...rects.map((r) => r.x + r.width)) - x,
      height: Math.max(...rects.map((r) => r.y + r.height)) - y,
    }
  })

  // Open, a shape closes only once the cursor is past a safety margin around
  // it, watched by the backend (the window ends at the shapes' edge). One
  // watch at a time: the notch, or a pin by its module id.
  let watching: string | null = null

  function stopWatching() {
    if (watching !== null) void invoke('cancel_leave')
    watching = null
  }

  function watch(who: string, area: Rect) {
    watching = who
    void invoke('watch_leave', { area })
  }

  function enter() {
    // Coming from a pin: that pin closes.
    if (watching !== null && watching !== 'notch') pinHover = null
    stopWatching()
    hovered = true
  }

  function leave() {
    if (expanded) watch('notch', notchArea)
    else hovered = false
  }

  function enterPin(module: string) {
    // Coming from the notch or another pin: that one closes.
    if (watching === 'notch') hovered = false
    stopWatching()
    pinHover = module
  }

  function leavePin(module: string) {
    const pin = pins.find((p) => p.section.module === module)
    if (pin?.open) watch(module, pin.rect)
    else if (pinHover === module) pinHover = null
  }

  function pointerLeft() {
    if (watching === 'notch') hovered = false
    else if (watching !== null && pinHover === watching) pinHover = null
    watching = null
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
  let spotlightTimer: ReturnType<typeof setTimeout> | undefined
  let pinAlertTimer: ReturnType<typeof setTimeout> | undefined

  function onContent(next: Content) {
    const alerts = newAlerts(allItems(content), allItems(next))
    content = next
    if (alerts.length === 0) return

    // One sound only, for the most important alert. A ringing one has its
    // own alarm (see below).
    const kinds = alerts.filter((i) => !i.ringing).map((i) => soundFor(i.tone))
    const kind = (['attention', 'error', 'done'] as const).find((k) => kinds.includes(k))
    if (kind && status.soundEnabled) playSound(kind)

    // A pinned module's alert opens its pin, the others open the notch.
    const pinnedIds = new Set(next.sections.filter((s) => s.pin).map((s) => s.module))
    const toNotch = alerts.map(moduleOf).filter((m) => !pinnedIds.has(m))
    const toPins = alerts.map(moduleOf).filter((m) => pinnedIds.has(m))
    if (toNotch.length > 0) {
      // An alert arriving while another shows adds its module to the open notch.
      alertModules = new Set([...(alerting ? alertModules : []), ...toNotch])
      alerting = true
      clearTimeout(alertTimer)
      alertTimer = setTimeout(() => (alerting = false), ALERT_MS)
    }
    if (toPins.length > 0) {
      pinAlerts = new Set([...(pinAlerting ? pinAlerts : []), ...toPins])
      pinAlerting = true
      clearTimeout(pinAlertTimer)
      pinAlertTimer = setTimeout(() => (pinAlerting = false), ALERT_MS)
    }
  }

  // The alarm of a ringing module, while it rings.
  const anyRinging = $derived(ringing(content.sections))
  $effect(() => {
    if (!anyRinging || !status.soundEnabled) return
    const ring = () => playSound('alarm')
    ring()
    const every = setInterval(ring, ALARM_EVERY_MS)
    const stop = setTimeout(() => clearInterval(every), ALARM_FOR_MS)
    return () => {
      clearInterval(every)
      clearTimeout(stop)
    }
  })

  // While the shapes animate, the clickable area covers each one's whole
  // way, from every place it went since the last settled state (a shape may
  // still be moving away from a state that lasted less than an animation)
  // to its new place: a pin pushed outwards by its neighbor opening is never
  // clipped on its way. Once the animations are over, only the new shapes.
  let resizeTimer: ReturnType<typeof setTimeout> | undefined
  let trails = new Map<string, Shape>()
  $effect(() => {
    if (!hitArea) return
    const next: (Shape & { key: string })[] = JSON.parse(hitArea)
    const settled = next.map(({ key, ...shape }) => shape)
    clearTimeout(resizeTimer)
    for (const { key, ...shape } of next) {
      const trail = trails.get(key)
      trails.set(key, trail ? around(trail, shape) : shape)
    }
    // The notch first: the cursor resistance follows the first shape.
    void invoke('set_hit_area', { shapes: [...settled, ...trails.values()] })
    resizeTimer = setTimeout(() => {
      trails = new Map(next.map(({ key, ...shape }) => [key, shape]))
      void invoke('set_hit_area', { shapes: settled })
    }, duration + 10)
  })

  $effect(() => {
    let noticeTimer: ReturnType<typeof setTimeout> | undefined
    const unlisten = [
      listen<Content>('content-changed', (e) => onContent(e.payload)),
      listen<Status>('status-changed', (e) => (status = e.payload)),
      // Past the safety margin of an open shape (see `watch`).
      listen('pointer-left', pointerLeft),
      listen<string>('spotlight', (e) => {
        spotlight = e.payload
        clearTimeout(spotlightTimer)
        spotlightTimer = setTimeout(() => (spotlight = null), SPOTLIGHT_MS)
      }),
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
      clearTimeout(pinAlertTimer)
      clearTimeout(spotlightTimer)
      clearTimeout(resizeTimer)
      unlisten.forEach((p) => p.then((off) => off()))
    }
  })
</script>

<svelte:window bind:innerWidth={windowWidth} bind:innerHeight={windowHeight} />

<Notch
  {cards}
  {start}
  foldedWidth={compactShape(status.edge, status.style).width}
  {duration}
  gap={CARD_GAP}
  allSections={compactSections}
  note={content.note}
  {moduleUis}
  {notice}
  {spotlight}
  edge={status.edge}
  style={status.style}
  {expanded}
  movable={status.movable}
  onclick={() => !status.movable && invoke('acknowledge')}
  onaction={(item, action) => void invoke('item_action', { item, action })}
  onpointerdown={startDrag}
  onpointermove={drag}
  onpointerup={endDrag}
  onenter={enter}
  onleave={leave}
/>

<!-- Pinned modules, next to the notch (DF-0012). -->
<div class="pins" class:movable={status.movable} style:--duration="{duration}ms">
  {#each pins as pin (pin.section.module)}
    <Pin
      section={pin.section}
      rect={pin.rect}
      corners={corners(status.edge, pin.radius, attached)}
      open={pin.open}
      spotlit={spotlight === pin.section.module}
      {vertical}
      {moduleUis}
      bind:measured={pinWidths[pin.section.module]}
      onenter={() => enterPin(pin.section.module)}
      onleave={() => leavePin(pin.section.module)}
      onclick={() => !status.movable && invoke('acknowledge')}
      onaction={(item, action) => void invoke('item_action', { item, action })}
    />
  {/each}
</div>

<style>
  /* Over the whole window; only the pins take the cursor. */
  .pins {
    position: fixed;
    inset: 0;
    pointer-events: none;
  }

  /* Move mode: pins follow the notch at once, like it. */
  .pins.movable :global(.pin) {
    transition: none;
  }
</style>
