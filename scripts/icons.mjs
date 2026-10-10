// Makes the app icons from the pixel art logo (assets/logo.png, 16 × 16,
// 8-bit RGBA): a charcoal square with well rounded smooth corners, and Molf's head
// (the logo's light pixels) in the middle, enlarged by a whole factor
// without smoothing so its pixels stay crisp. It writes the 1024 px source
// (assets/icon.png) and the sizes Windows shows; `pnpm tauri icon
// assets/icon.png` makes the other formats (macOS, Windows Store) from that
// source, then run this script again: Tauri's resizing blurs the pixels.
//
//   node scripts/icons.mjs
import { readFileSync, writeFileSync } from 'node:fs'
import { deflateSync, inflateSync } from 'node:zlib'

const LOGO = 'assets/logo.png'
const ICONS = 'src-tauri/icons'

function decode(file) {
  const buf = readFileSync(file)
  let pos = 8
  let width, height, depth, type
  const idat = []
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos)
    const kind = buf.toString('ascii', pos + 4, pos + 8)
    const data = buf.subarray(pos + 8, pos + 8 + len)
    if (kind === 'IHDR') {
      width = data.readUInt32BE(0)
      height = data.readUInt32BE(4)
      depth = data[8]
      type = data[9]
    } else if (kind === 'IDAT') idat.push(data)
    pos += 12 + len
  }
  if (depth !== 8 || type !== 6) throw new Error(`${file}: expected an 8-bit RGBA PNG`)
  const stride = width * 4
  const raw = inflateSync(Buffer.concat(idat))
  const rows = []
  let prev = Buffer.alloc(stride)
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)]
    const line = Buffer.from(raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1)))
    for (let i = 0; i < stride; i++) {
      const a = i >= 4 ? line[i - 4] : 0
      const b = prev[i]
      const c = i >= 4 ? prev[i - 4] : 0
      const p = a + b - c
      const pa = Math.abs(p - a)
      const pb = Math.abs(p - b)
      const pc = Math.abs(p - c)
      const paeth = pa <= pb && pa <= pc ? a : pb <= pc ? b : c
      line[i] = (line[i] + [0, a, b, (a + b) >> 1, paeth][filter]) & 255
    }
    rows.push(line)
    prev = line
  }
  return { width, height, rows }
}

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
  return c >>> 0
})
const crc = (bytes) => {
  let c = 0xffffffff
  for (const x of bytes) c = crcTable[(c ^ x) & 255] ^ (c >>> 8)
  return (c ^ 0xffffffff) >>> 0
}
const chunk = (kind, data) => {
  const length = Buffer.alloc(4)
  length.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(kind), data])
  const sum = Buffer.alloc(4)
  sum.writeUInt32BE(crc(body))
  return Buffer.concat([length, body, sum])
}

// An RGBA picture of `size` × `size` pixels as a PNG.
function encode(size, pixels) {
  const out = Buffer.alloc((size * 4 + 1) * size)
  for (let y = 0; y < size; y++) {
    pixels.copy(out, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4)
  }
  const header = Buffer.alloc(13)
  header.writeUInt32BE(size, 0)
  header.writeUInt32BE(size, 4)
  header[8] = 8
  header[9] = 6
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(out, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

// The square's color, Molf's, the corners' radius (a share of the size) and
// how much of the width Molf takes, about.
const BACKGROUND = [0x26, 0x26, 0x26]
const FOREGROUND = [0xf2, 0xf2, 0xf2]
const RADIUS = 0.32
const FILL = 0.62

// Molf's head: the logo's light pixels, as "x,y" from the top left of their
// box, and the box's size.
function headOf({ rows }) {
  const cells = []
  rows.forEach((row, y) => {
    for (let x = 0; x < 16; x++) {
      if (row[x * 4 + 3] > 128 && row[x * 4] > 128) cells.push([x, y])
    }
  })
  const left = Math.min(...cells.map(([x]) => x))
  const top = Math.min(...cells.map(([, y]) => y))
  return {
    cells: new Set(cells.map(([x, y]) => `${x - left},${y - top}`)),
    width: Math.max(...cells.map(([x]) => x)) - left + 1,
    height: Math.max(...cells.map(([, y]) => y)) - top + 1,
  }
}

// The icon at `size` px: the square's edge smoothed by 4 × 4 samples per
// pixel, Molf drawn with whole pixels (each `cell` px), centered.
function icon(head, size) {
  const pixels = Buffer.alloc(size * size * 4)
  const radius = size * RADIUS
  const cell = Math.max(1, Math.round((size * FILL) / head.width))
  const left = Math.round((size - head.width * cell) / 2)
  const top = Math.round((size - head.height * cell) / 2)
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let inside = 0
      for (let sy = 0; sy < 4; sy++) {
        for (let sx = 0; sx < 4; sx++) {
          const fx = x + (sx + 0.5) / 4
          const fy = y + (sy + 0.5) / 4
          const cx = Math.min(Math.max(fx, radius), size - radius)
          const cy = Math.min(Math.max(fy, radius), size - radius)
          if ((fx - cx) ** 2 + (fy - cy) ** 2 <= radius * radius) inside++
        }
      }
      const molf =
        x >= left && y >= top && head.cells.has(`${Math.floor((x - left) / cell)},${Math.floor((y - top) / cell)}`)
      const color = molf ? FOREGROUND : BACKGROUND
      const i = (y * size + x) * 4
      pixels[i] = color[0]
      pixels[i + 1] = color[1]
      pixels[i + 2] = color[2]
      pixels[i + 3] = Math.round((inside / 16) * 255)
    }
  }
  return encode(size, pixels)
}

// A .ico of PNG entries (Windows Vista and later).
function ico(pngs) {
  const header = Buffer.alloc(6)
  header.writeUInt16LE(1, 2)
  header.writeUInt16LE(pngs.length, 4)
  let offset = 6 + 16 * pngs.length
  const entries = pngs.map((png) => {
    const size = png.readUInt32BE(16)
    const entry = Buffer.alloc(16)
    entry[0] = size >= 256 ? 0 : size
    entry[1] = size >= 256 ? 0 : size
    entry.writeUInt16LE(1, 4)
    entry.writeUInt16LE(32, 6)
    entry.writeUInt32LE(png.length, 8)
    entry.writeUInt32LE(offset, 12)
    offset += png.length
    return entry
  })
  return Buffer.concat([header, ...entries, ...pngs])
}

const logo = decode(LOGO)
if (logo.width !== 16 || logo.height !== 16) throw new Error(`${LOGO}: expected 16 × 16`)
const head = headOf(logo)
writeFileSync('assets/icon.png', icon(head, 1024))
writeFileSync(`${ICONS}/icon.png`, icon(head, 512))
writeFileSync(`${ICONS}/32x32.png`, icon(head, 32))
writeFileSync(`${ICONS}/64x64.png`, icon(head, 64))
writeFileSync(`${ICONS}/128x128.png`, icon(head, 128))
writeFileSync(`${ICONS}/128x128@2x.png`, icon(head, 256))
// 16, 32, 48, 64 and 256 px: the sizes Windows picks from.
writeFileSync(`${ICONS}/icon.ico`, ico([16, 32, 48, 64, 256].map((size) => icon(head, size))))
console.log('icons made from', LOGO)
