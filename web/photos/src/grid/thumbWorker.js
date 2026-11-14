// Fetches and decodes the grid's thumbnails off the main thread, so scrolling never waits on a
// JPEG. Small tiles come from the server's bulk endpoint (the 320 px tiles the iPhone app uses,
// up to 300 per request) and are kept in Cache Storage, so coming back to them never touches the
// network. Big tiles (3 or 5 columns on a wide screen) are cut from the 2048 px preview once, and
// that cut, already at the tile's size, is kept in Cache Storage too, so it isn't decoded from
// 2048 px again. Every image is cropped to a square and shrunk to the size it's drawn at before
// it's handed back, along with a 40 px copy the grid keeps for good as a stand-in.
//
// Messages in:  { type: 'want', items: [[id, checksum, path, isVideo, size], …] }
//               the list replaces whatever was still queued (most important first), so photos
//               flung past are dropped instead of loaded.
// Messages out: { type: 'thumb', id, size, bitmap, tiny }  (transferred; bitmap null if it failed)

const CACHE = 'tpphotos-thumbs-v1'
const BULK = 150 // per request; the server allows 300, smaller batches come back sooner
const BULK_PARALLEL = 3
const PREVIEW_PARALLEL = 4
const SMALL_MAX = 320 // above this the preview is used
const TINY = 40

let smallQueue = []
let bigQueue = []
const inFlight = new Set() // `${id}:${size}`
let bulkRunning = 0
let previewRunning = 0
let cachePromise = null

function cache() {
  if (!cachePromise) cachePromise = (self.caches ? caches.open(CACHE) : Promise.resolve(null)).catch(() => null)
  return cachePromise
}

const thumbKey = item => `/photos/__thumbs/${item.id}?v=${item.checksum}`
const tileKey = item => `/photos/__tiles/${item.id}?v=${item.checksum}&s=${item.size}`

self.onmessage = e => {
  const msg = e.data
  if (msg.type !== 'want') return
  smallQueue = []
  bigQueue = []
  for (const [id, checksum, path, isVideo, size] of msg.items) {
    if (inFlight.has(`${id}:${size}`)) continue
    ;(size > SMALL_MAX ? bigQueue : smallQueue).push({ id, checksum, path, isVideo, size })
  }
  pump()
}

function pump() {
  while (bigQueue.length && previewRunning < PREVIEW_PARALLEL) loadBig(bigQueue.shift())
  while (smallQueue.length && bulkRunning < BULK_PARALLEL) loadSmall(smallQueue.splice(0, BULK))
}

function send(item, result) {
  inFlight.delete(`${item.id}:${item.size}`)
  const bitmap = result?.bitmap || null
  const tiny = result?.tiny || null
  self.postMessage({ type: 'thumb', id: item.id, size: item.size, bitmap, tiny }, [bitmap, tiny].filter(Boolean))
}

async function loadSmall(batch) {
  bulkRunning++
  for (const item of batch) inFlight.add(`${item.id}:${item.size}`)
  try {
    const store = await cache()
    // What's already on this computer is decoded straight away; the rest goes in one request.
    const hits = await Promise.all(batch.map(item => (store ? store.match(thumbKey(item)).catch(() => null) : null)))
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
        store?.put(thumbKey(item), new Response(blob, { headers: { 'Content-Type': 'image/jpeg' } })).catch(() => {})
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

async function loadBig(item) {
  previewRunning++
  inFlight.add(`${item.id}:${item.size}`)
  try {
    const store = await cache()
    const hit = store ? await store.match(tileKey(item)).catch(() => null) : null
    if (hit) {
      send(item, await decode(await hit.blob(), item.size))
      return
    }
    const params = new URLSearchParams({ path: item.path, location: 'external', v: item.checksum })
    const endpoint = item.isVideo ? 'poster' : 'preview'
    const res = await fetch(`/store/api/files/${endpoint}?${params}`)
    if (!res.ok) throw new Error(String(res.status))
    send(item, await decode(await res.blob(), item.size, store ? blob => store.put(tileKey(item), new Response(blob, { headers: { 'Content-Type': 'image/jpeg' } })) : null))
  } catch {
    send(item, null)
  } finally {
    previewRunning--
    pump()
  }
}

const canvases = new Map()
function canvasOf(size) {
  let canvas = canvases.get(size)
  if (!canvas) {
    canvas = new OffscreenCanvas(size, size)
    canvases.set(size, canvas)
  }
  return canvas
}

// A square of `size` px cut from the middle of the image, like the server's thumbnails, and the
// 40 px stand-in. `keep(blob)` gets the cut as a JPEG to store, for the big tiles.
async function decode(blob, size, keep = null) {
  let source
  try {
    source = await createImageBitmap(blob)
  } catch {
    return null
  }
  if (typeof OffscreenCanvas === 'undefined') return { bitmap: source, tiny: null }
  const side = Math.min(source.width, source.height)
  const sx = (source.width - side) / 2
  const sy = (source.height - side) / 2
  // The drawing below is synchronous, so the shared canvases can't be used by two decodes at once.
  const draw = s => {
    const canvas = canvasOf(s)
    const ctx = canvas.getContext('2d')
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(source, sx, sy, side, side, 0, 0, s, s)
    return canvas
  }
  const tiny = draw(TINY).transferToImageBitmap()
  let bitmap
  if (side <= size * 1.05 && source.width === source.height) {
    bitmap = source
  } else {
    if (keep) {
      // Its own canvas: encoding is asynchronous.
      const own = new OffscreenCanvas(size, size)
      const ctx = own.getContext('2d')
      ctx.imageSmoothingQuality = 'high'
      ctx.drawImage(source, sx, sy, side, side, 0, 0, size, size)
      own.convertToBlob({ type: 'image/jpeg', quality: 0.88 }).then(keep).catch(() => {})
    }
    bitmap = draw(size).transferToImageBitmap()
    source.close()
  }
  return { bitmap, tiny }
}
