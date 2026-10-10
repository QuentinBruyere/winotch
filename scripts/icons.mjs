// Makes the app icons from the pixel art logo (assets/logo.png, 16 × 16,
// 8-bit RGBA): each size an exact multiple of it, enlarged without smoothing
// so the pixels stay crisp. Run `pnpm tauri icon assets/icon.png` first for
// the other formats (macOS, Windows Store), then this script: it replaces
// the sizes Windows shows and the 1024 px source.
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

// The logo `factor` times bigger, as a PNG.
function enlarged({ width, height, rows }, factor) {
  const w = width * factor
  const h = height * factor
  const out = Buffer.alloc((w * 4 + 1) * h)
  for (let y = 0; y < h; y++) {
    const source = rows[Math.floor(y / factor)]
    for (let x = 0; x < w; x++) {
      const from = Math.floor(x / factor) * 4
      source.copy(out, y * (w * 4 + 1) + 1 + x * 4, from, from + 4)
    }
  }
  const header = Buffer.alloc(13)
  header.writeUInt32BE(w, 0)
  header.writeUInt32BE(h, 4)
  header[8] = 8
  header[9] = 6
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(out, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ])
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
writeFileSync('assets/icon.png', enlarged(logo, 64))
writeFileSync(`${ICONS}/icon.png`, enlarged(logo, 32))
writeFileSync(`${ICONS}/32x32.png`, enlarged(logo, 2))
writeFileSync(`${ICONS}/64x64.png`, enlarged(logo, 4))
writeFileSync(`${ICONS}/128x128.png`, enlarged(logo, 8))
writeFileSync(`${ICONS}/128x128@2x.png`, enlarged(logo, 16))
// 16, 32, 48, 64 and 256 px: the sizes Windows picks from.
writeFileSync(`${ICONS}/icon.ico`, ico([1, 2, 3, 4, 16].map((f) => enlarged(logo, f))))
console.log('icons made from', LOGO)
