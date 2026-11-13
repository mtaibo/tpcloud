// Fetches and decodes the grid's thumbnails off the main thread, so scrolling never waits on a
// JPEG. Small tiles come from the server's bulk endpoint (the 320 px tiles the iPhone app uses,
// up to 300 per request) and are kept in Cache Storage, so a second visit doesn't touch the
// network. Big tiles (3 columns on a wide screen) come from the 2048 px preview instead, which
// the browser's HTTP cache keeps (its URL is versioned by checksum). Every image is cropped to a
// square and shrunk to the size it's drawn at before it's handed back, so the main thread only
// ever draws small bitmaps.
//
// Messages in:  { type: 'want', size, items: [[id, checksum, path, isVideo], …] }
//               the list replaces whatever was still queued (most important first), so photos
//               flung past are dropped instead of loaded.
// Messages out: { type: 'thumb', id, size, bitmap }  (bitmap transferred; null if it failed)

const CACHE = 'tpphotos-thumbs-v1'
const BULK = 150 // per request; the server allows 300, smaller batches come back sooner
const BULK_PARALLEL = 3
const PREVIEW_PARALLEL = 4
const SMALL_MAX = 320 // above this the preview is used

let queue = [] // [{ id, checksum, path, isVideo, size }]
const inFlight = new Set() // `${id}:${size}`
let bulkRunning = 0
let previewRunning = 0
let cachePromise = null

function cache() {
  if (!cachePromise) cachePromise = (self.caches ? caches.open(CACHE) : Promise.resolve(null)).catch(() => null)
  return cachePromise
}

const cacheKey = item => `/photos/__thumbs/${item.id}?v=${item.checksum}`

self.onmessage = e => {
  const msg = e.data
  if (msg.type !== 'want') return
  queue = msg.items
    .map(([id, checksum, path, isVideo]) => ({ id, checksum, path, isVideo, size: msg.size }))
    .filter(item => !inFlight.has(`${item.id}:${item.size}`))
  pump()
}

function pump() {
  while (queue.length && queue[0].size > SMALL_MAX && previewRunning < PREVIEW_PARALLEL) {
    loadPreview(queue.shift())
  }
  while (queue.length && queue[0].size <= SMALL_MAX && bulkRunning < BULK_PARALLEL) {
    const batch = queue.splice(0, BULK)
    loadSmall(batch)
  }
}

function send(item, bitmap) {
  inFlight.delete(`${item.id}:${item.size}`)
  self.postMessage({ type: 'thumb', id: item.id, size: item.size, bitmap }, bitmap ? [bitmap] : [])
}

async function loadSmall(batch) {
  bulkRunning++
  for (const item of batch) inFlight.add(`${item.id}:${item.size}`)
  try {
    const store = await cache()
    // What's already on this computer is decoded straight away; the rest goes in one request.
    const hits = await Promise.all(batch.map(item => (store ? store.match(cacheKey(item)).catch(() => null) : null)))
    const missing = []
    await Promise.all(batch.map(async (item, i) => {
      if (hits[i]) send(item, await decode(await hits[i].blob(), item.size))
      else missing.push(item)
    }))
    if (missing.length) {
      const res = await fetch('/store/api/photos/thumbnails', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ids: missing.map(m => m.id) }),
      })
      if (!res.ok) throw new Error(String(res.status))
      const parts = unpack(await res.arrayBuffer())
      await Promise.all(missing.map(async item => {
        const bytes = parts.get(item.id)
        if (!bytes || !bytes.byteLength) { send(item, null); return }
        const blob = new Blob([bytes], { type: 'image/jpeg' })
        store?.put(cacheKey(item), new Response(blob, { headers: { 'Content-Type': 'image/jpeg' } })).catch(() => {})
        send(item, await decode(blob, item.size))
      }))
    }
  } catch {
    for (const item of batch) if (inFlight.has(`${item.id}:${item.size}`)) send(item, null)
  } finally {
    bulkRunning--
    pump()
  }
}

// [4-byte id][4-byte length][JPEG]… big-endian, length 0 when the server has none.
function unpack(buffer) {
  const view = new DataView(buffer)
  const out = new Map()
  let at = 0
  while (at + 8 <= buffer.byteLength) {
    const id = view.getUint32(at)
    const length = view.getUint32(at + 4)
    at += 8
    out.set(id, new Uint8Array(buffer, at, length))
    at += length
  }
  return out
}

async function loadPreview(item) {
  previewRunning++
  inFlight.add(`${item.id}:${item.size}`)
  try {
    const params = new URLSearchParams({ path: item.path, location: 'external', v: item.checksum })
    const endpoint = item.isVideo ? 'poster' : 'preview'
    const res = await fetch(`/store/api/files/${endpoint}?${params}`)
    if (!res.ok) throw new Error(String(res.status))
    send(item, await decode(await res.blob(), item.size))
  } catch {
    send(item, null)
  } finally {
    previewRunning--
    pump()
  }
}

const canvases = new Map()

// A square of `size` px cut from the middle of the image, like the server's thumbnails.
async function decode(blob, size) {
  try {
    const source = await createImageBitmap(blob)
    const side = Math.min(source.width, source.height)
    if (typeof OffscreenCanvas === 'undefined' || (side <= size * 1.15 && source.width === source.height)) return source
    let canvas = canvases.get(size)
    if (!canvas) {
      canvas = new OffscreenCanvas(size, size)
      canvases.set(size, canvas)
    }
    const ctx = canvas.getContext('2d')
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(source, (source.width - side) / 2, (source.height - side) / 2, side, side, 0, 0, size, size)
    source.close()
    return canvas.transferToImageBitmap()
  } catch {
    return null
  }
}
