// The grid's decoded thumbnails, in memory, at the sizes they're drawn at. The worker fetches and
// shrinks them; this keeps the most recently used ones up to a memory budget, plus a 40 px copy of
// every photo seen so far that's never dropped, so a tile coming back on screen shows at once
// (blurry for a moment) instead of empty.

const SMALL_MAX = 320
const TINY = 40

// The size a tile is decoded at: its size on screen in pixels, rounded up a little so a few
// nearby sizes share bitmaps. Up to 320 px it's cut from the server's 320 px thumbnail, above
// that from the 2048 px preview.
export function sizeFor(tileCss, dpr) {
  const px = tileCss * dpr
  if (px <= TINY * 1.15) return TINY // the 61-column grid draws the 40 px copies kept anyway
  if (px <= SMALL_MAX * 1.12) return Math.min(SMALL_MAX, Math.max(64, Math.ceil(px / 32) * 32))
  return Math.min(1024, Math.ceil(px / 64) * 64)
}

export class ThumbCache {
  constructor(onArrive) {
    this.onArrive = onArrive // (id) => void
    this.budget = (window.innerWidth < 768 ? 160 : 320) * 1024 * 1024 // bytes of decoded pixels
    this.map = new Map() // `${id}:${size}` -> ImageBitmap, least recently used first
    this.sizes = new Map() // id -> Set of sizes in `map`
    this.tiny = new Map() // id -> 40 px ImageBitmap, kept for good
    this.bytes = 0
    this.failed = new Set()
    this.pinned = new Set() // ids on screen, never evicted
    this.worker = new Worker(new URL('./thumbWorker.js', import.meta.url), { type: 'module' })
    this.worker.onmessage = e => this.receive(e.data)
    this.lastWant = ''
  }

  receive({ id, size, bitmap, tiny }) {
    if (tiny) {
      if (this.tiny.has(id)) tiny.close()
      else this.tiny.set(id, tiny)
    }
    const key = `${id}:${size}`
    if (!bitmap) {
      this.failed.add(key)
      if (tiny) this.onArrive(id)
      return
    }
    const old = this.map.get(key)
    if (old) { this.bytes -= old.width * old.height * 4; old.close() }
    this.map.set(key, bitmap)
    let set = this.sizes.get(id)
    if (!set) this.sizes.set(id, (set = new Set()))
    set.add(size)
    this.bytes += bitmap.width * bitmap.height * 4
    this.evict()
    this.onArrive(id)
  }

  evict() {
    if (this.bytes <= this.budget) return
    for (const [key, bitmap] of this.map) {
      if (this.bytes <= this.budget * 0.85) break
      const id = Number(key.slice(0, key.indexOf(':')))
      if (this.pinned.has(id)) continue
      this.map.delete(key)
      this.sizes.get(id)?.delete(Number(key.slice(key.indexOf(':') + 1)))
      this.bytes -= bitmap.width * bitmap.height * 4
      bitmap.close()
    }
  }

  /** The best bitmap there is for a photo: the size asked for, else the closest one, else the 40 px copy. */
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
    for (const s of this.sizes.get(id) || []) {
      // A bigger one scales down fine; a smaller one is a blurry stand-in.
      const score = s >= size ? s - size : (size - s) * 4
      if (score < bestScore) { best = this.map.get(`${id}:${s}`); bestScore = score }
    }
    return best || this.tiny.get(id) || null
  }

  has(id, size) {
    if (size === TINY && this.tiny.has(id)) return true
    return this.map.has(`${id}:${size}`) || this.failed.has(`${id}:${size}`)
  }

  /** Asks the worker for the photos still missing, most important first. */
  want(items, size) {
    const list = []
    const big = size > SMALL_MAX
    // Big tiles: the 320 px version first, which is on this computer already and comes at once,
    // then the sharp one.
    if (big) for (const p of items) {
      if (!this.has(p.id, size) && !this.has(p.id, SMALL_MAX)) list.push([p.id, p.checksum, p.path, p.isVideo, SMALL_MAX])
    }
    for (const p of items) {
      if (!this.has(p.id, size)) list.push([p.id, p.checksum, p.path, p.isVideo, size])
    }
    const signature = `${size}|${list.length}|${list[0]?.[0]}|${list.at(-1)?.[0]}`
    if (signature === this.lastWant) return
    this.lastWant = signature
    this.worker.postMessage({ type: 'want', items: list })
  }

  pin(ids) {
    this.pinned = ids
  }

  destroy() {
    this.worker.terminate()
    for (const b of this.map.values()) b.close()
    for (const b of this.tiny.values()) b.close()
    this.map.clear()
    this.tiny.clear()
  }
}
