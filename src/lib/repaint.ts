// WebView2 sometimes draws a frame with a white background instead of a
// transparent one: the white shows as an edge around the notch (in the
// window region's margin) until the next frame, and a still notch draws
// none. A barely different opacity for one frame forces a fresh one.
export function repaint() {
  const root = document.documentElement
  root.style.opacity = '0.999'
  requestAnimationFrame(() => requestAnimationFrame(() => (root.style.opacity = '')))
}

// After an event that may cause it (another window opens, the notch shows
// again), the bad frame can come a little later: repaint a few times.
const DELAYS_MS = [0, 300, 1000]

export function repaintSoon() {
  for (const delay of DELAYS_MS) setTimeout(repaint, delay)
}

// Safety net for causes nobody announced: a white edge lasts at most this
// long. Costs one tiny frame every 2 s, only while the notch is visible.
export const SAFETY_REPAINT_MS = 2000
