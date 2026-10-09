import type { Animation, Companion, Frame } from './companion'

// A pack (ADR-0015) made from a built-in companion: its distinct pictures on
// a PNG sheet and the companion.json saying which plays when. The template
// users start from to draw their own (DF-0025); made from the drawings
// themselves, it stays up to date.
export async function templatePack(
  companion: Companion,
  name: string,
): Promise<{ sheet: number[]; manifest: string }> {
  const frames: Frame[] = []
  const known = new Map<string, number>()
  const index = (frame: Frame) => {
    const key = frame.join('\n')
    let i = known.get(key)
    if (i === undefined) {
      i = frames.length
      frames.push(frame)
      known.set(key, i)
    }
    return i
  }
  const refs = (list: Frame[]) => ranges(list.map(index))
  const spec = (animation: Animation) => ({
    fps: animation.fps,
    ...(animation.intro?.length ? { intro: refs(animation.intro) } : {}),
    frames: refs(animation.frames),
    ...(animation.scenes ? { scenes: animation.scenes.map(refs) } : {}),
  })
  const animations: Record<string, unknown> = {}
  for (const [key, animation] of Object.entries(companion.animations)) animations[key] = spec(animation)
  for (const [key, animation] of Object.entries(companion.moods)) {
    if (animation) animations[key] = spec(animation)
  }

  const { width, height } = companion
  const columns = Math.min(16, frames.length)
  const canvas = document.createElement('canvas')
  canvas.width = columns * width
  canvas.height = Math.ceil(frames.length / columns) * height
  const context = canvas.getContext('2d')!
  // A pixel of the sheet: any opaque color, the pack is tinted.
  context.fillStyle = '#000'
  frames.forEach((frame, i) => {
    const left = (i % columns) * width
    const top = Math.floor(i / columns) * height
    frame.forEach((row, y) => {
      ;[...row].forEach((char, x) => {
        if (char !== '.') context.fillRect(left + x, top + y, 1, 1)
      })
    })
  })
  const blob = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('no PNG'))), 'image/png'),
  )
  const manifest = {
    name,
    sheet: 'sheet.png',
    frameWidth: width,
    frameHeight: height,
    colors: 'tint',
    animations,
  }
  return {
    sheet: Array.from(new Uint8Array(await blob.arrayBuffer())),
    manifest: JSON.stringify(manifest, null, 2),
  }
}

// Picture numbers, runs of 3 or more written as a range "a-b".
function ranges(indices: number[]): (number | string)[] {
  const out: (number | string)[] = []
  let i = 0
  while (i < indices.length) {
    let j = i
    while (j + 1 < indices.length && indices[j + 1] === indices[j] + 1) j++
    if (j - i >= 2) out.push(`${indices[i]}-${indices[j]}`)
    else out.push(...indices.slice(i, j + 1))
    i = j + 1
  }
  return out
}
