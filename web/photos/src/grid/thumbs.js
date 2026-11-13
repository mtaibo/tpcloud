// The grid's decoded thumbnails, in memory, at the sizes they're drawn at. The worker fetches and
// shrinks them; this keeps the most recently used ones up to a memory budget and tells the grid
// when one arrives.

const BUDGET = 256 * 1024 * 1024 // bytes of decoded pixels kept around

// The sizes a tile is decoded at: the smallest one at least as big as the tile on screen.
const SIZES = [96, 192, 320, 640, 1024]

export function sizeFor(tileCss, dpr) {
  const px = tileCss * dpr
  return SIZES.find(s => s >= px * 0.95) || SIZES.at(-1)
}

export class ThumbCache {
  constructor(onArrive) {
    this.onArrive = onArrive // (id) => void
    this.map = new Map() // `${id}:${size}` -> ImageBitmap, oldest first
    this.bytes = 0
    this.failed = new Set()
    this.pinned = new Set() // ids on screen, never evicted
    this.worker = new Worker(new URL('./thumbWorker.js', import.meta.url), { type: 'module' })
    this.worker.onmessage = e => this.receive(e.data)
    this.lastWant = ''
  }

  receive({ id, size, bitmap }) {
    const key = `${id}:${size}`
    if (!bitmap) { this.failed.add(key); return }
    const old = this.map.get(key)
    if (old) { this.bytes -= old.width * old.height * 4; old.close() }
    this.map.set(key, bitmap)
    this.bytes += bitmap.width * bitmap.height * 4
    this.evict()
    this.onArrive(id)
  }

  evict() {
    if (this.bytes <= BUDGET) return
    for (const [key, bitmap] of this.map) {
      if (this.bytes <= BUDGET * 0.85) break
      if (this.pinned.has(Number(key.slice(0, key.indexOf(':'))))) continue
      this.map.delete(key)
      this.bytes -= bitmap.width * bitmap.height * 4
      bitmap.close()
    }
  }

  /** The best bitmap there is for a photo: the size asked for, else the closest one. */
  get(id, size) {
    const key = `${id}:${size}`
    const exact = this.map.get(key)
    if (exact) {
      this.map.delete(key) // most recently used goes last
      this.map.set(key, exact)
      return exact
    }
    let best = null
    let bestScore = Infinity
    for (const s of SIZES) {
      const b = this.map.get(`${id}:${s}`)
      if (!b) continue
      // A bigger one scales down fine; a smaller one is a blurry stand-in.
      const score = s >= size ? s - size : (size - s) * 4
      if (score < bestScore) { best = b; bestScore = score }
    }
    return best
  }

  has(id, size) {
    return this.map.has(`${id}:${size}`) || this.failed.has(`${id}:${size}`)
  }

  /** Asks the worker for the photos still missing, most important first. */
  want(items, size) {
    const list = []
    for (const p of items) {
      if (!this.has(p.id, size)) list.push([p.id, p.checksum, p.path, p.isVideo])
    }
    const signature = `${size}|${list.length}|${list[0]?.[0]}|${list.at(-1)?.[0]}`
    if (signature === this.lastWant) return
    this.lastWant = signature
    this.worker.postMessage({ type: 'want', size, items: list })
  }

  pin(ids) {
    this.pinned = ids
  }

  destroy() {
    this.worker.terminate()
    for (const b of this.map.values()) b.close()
    this.map.clear()
  }
}
