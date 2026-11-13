import { ThumbCache, sizeFor } from './thumbs.js'

// The gallery's grid, built like the iPhone app's: no element per photo (11,000 of them made
// the browser stutter), but one <canvas> per row on screen, drawn from small decoded thumbnails
// and recycled as rows scroll in and out. The page scrolls natively, so the browser's compositor
// moves it at the screen's rate whatever the main thread is doing.
//
// The app's densities (3, 5, 9, 15 and 31 columns) and one more, 61, all odd; newest photo at the bottom,
// rows cut from the newest end so the partial row is the oldest one, at the top.
//
// Zooming works like the app's `PhotoGridZoomController` (and Photos): the grid being left and the
// new one, laid out once at rest underneath it, scale together about the middle of the grid
// across and, down, the height that lines the photo under the fingers' row up in both. Every
// column count is odd, so the columns of the two grids line up too, and they cross-fade with the
// pinch's progress, so each photo fades into its new cell instead of travelling there. One pinch
// can sweep through several levels; letting go springs on (or back) with the fingers' speed; past
// the densest and the largest levels the grid stretches and springs back.

export const LEVELS = [3, 5, 9, 15, 31, 61]

/** The densest level a grid this wide offers: 61 columns only on a big screen; on a phone or a
 *  tablet held upright the tiles would be a few pixels across. */
export function maxLevelFor(width) {
  return width >= 900 ? LEVELS.length - 1 : LEVELS.length - 2
}

const gapFor = cols => (cols >= 15 ? 1 : 2)

class Layout {
  constructor(count, cols, width, top, bottom) {
    this.count = count
    this.cols = cols
    this.width = width
    this.top = top
    this.gap = gapFor(cols)
    this.tile = Math.max(1, (width - this.gap * (cols - 1)) / cols)
    this.pitch = this.tile + this.gap
    this.rows = Math.ceil(count / cols)
    this.height = top + Math.max(0, this.rows * this.pitch - this.gap) + bottom
  }

  rowOf(i) { return this.rows - 1 - Math.floor((this.count - 1 - i) / this.cols) }
  rowStart(r) { return r === 0 ? 0 : this.count - (this.rows - r) * this.cols }
  rowEnd(r) { return this.count - (this.rows - 1 - r) * this.cols } // exclusive
  rowTop(r) { return this.top + r * this.pitch }
  x(i) { return (i - this.rowStart(this.rowOf(i))) * this.pitch }
  y(i) { return this.rowTop(this.rowOf(i)) }

  /** The photo at a point in the layout, the nearest one if the point is in a gap or off the end. */
  itemAt(x, y) {
    if (!this.count) return null
    const r = Math.min(this.rows - 1, Math.max(0, Math.floor((y - this.top) / this.pitch)))
    const start = this.rowStart(r)
    const end = this.rowEnd(r)
    const c = Math.max(0, Math.floor(x / this.pitch))
    const i = Math.min(end - 1, start + c)
    const v = Math.min(1, Math.max(0, (y - this.rowTop(r)) / this.tile))
    return { index: i, v }
  }
}

class Layer {
  constructor(grid, level) {
    this.grid = grid
    this.level = level
    this.el = document.createElement('div')
    this.el.className = 'pg-layer'
    this.rows = new Map() // row -> canvas
    // While zooming: { oy, k, ty }, the layer scaled by k about layout height oy, then moved ty.
    this.xf = null
    grid.spacer.insertBefore(this.el, grid.chipsEl) // the chips stay on top
    this.relayout()
  }

  relayout() {
    const g = this.grid
    this.layout = new Layout(g.items.length, LEVELS[this.level], g.width, g.insetTop, g.insetBottom)
    this.size = sizeFor(this.layout.tile, g.dpr)
    this.el.style.width = `${g.width}px`
    this.el.style.height = `${this.layout.height}px`
    this.releaseAll()
  }

  /** The layout heights on screen, with a margin. */
  visibleRange(margin) {
    const g = this.grid
    const xf = this.xf
    if (!xf) return [g.scrollTop - g.height * margin, g.scrollTop + g.height * (1 + margin)]
    // On screen at v: oy + (y - oy) * k + ty - scrollTop = v.
    const at = v => xf.oy + (v + g.scrollTop - xf.ty - xf.oy) / xf.k
    const y0 = at(0)
    const y1 = at(g.height)
    const pad = (y1 - y0) * margin
    return [y0 - pad, y1 + pad]
  }

  clearTransform() {
    this.xf = null
    const s = this.el.style
    s.transform = ''
    s.transformOrigin = ''
    s.opacity = ''
    s.maskImage = s.webkitMaskImage = ''
  }

  releaseAll() {
    for (const canvas of this.rows.values()) this.grid.releaseCanvas(canvas)
    this.rows.clear()
  }

  /** Keeps a canvas for every row between layout y0 and y1 and drops the others. */
  ensureRows(y0, y1) {
    const L = this.layout
    if (!L.rows) { this.releaseAll(); return }
    const r0 = Math.max(0, Math.floor((y0 - L.top) / L.pitch))
    const r1 = Math.min(L.rows - 1, Math.floor((y1 - L.top) / L.pitch))
    for (const [r, canvas] of this.rows) {
      if (r < r0 || r > r1) { this.grid.releaseCanvas(canvas); this.rows.delete(r) }
    }
    for (let r = r0; r <= r1; r++) {
      if (this.rows.has(r)) continue
      const canvas = this.grid.takeCanvas(this.el)
      this.rows.set(r, canvas)
      this.drawRow(r, canvas)
    }
  }

  drawRow(r, canvas) {
    const g = this.grid
    const L = this.layout
    const dpr = g.dpr
    const w = Math.ceil(g.width * dpr)
    const h = Math.ceil(L.tile * dpr)
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w
      canvas.height = h
      canvas.style.width = `${g.width}px`
      canvas.style.height = `${L.tile}px`
    }
    canvas.style.transform = `translate3d(0, ${L.rowTop(r)}px, 0)`
    const ctx = canvas.getContext('2d')
    ctx.clearRect(0, 0, w, h)
    ctx.imageSmoothingEnabled = true
    ctx.imageSmoothingQuality = 'medium'
    const start = L.rowStart(r)
    const end = L.rowEnd(r)
    const badges = L.tile >= 56
    for (let i = start; i < end; i++) {
      const p = g.items[i]
      const x0 = Math.round((i - start) * L.pitch * dpr)
      const x1 = Math.round(((i - start) * L.pitch + L.tile) * dpr)
      const bitmap = g.thumbs.get(p.id, this.size)
      if (bitmap) ctx.drawImage(bitmap, x0, 0, x1 - x0, h)
      else { ctx.fillStyle = g.placeholder; ctx.fillRect(x0, 0, x1 - x0, h) }
      if (badges && (p.isVideo || p.favorite)) drawBadges(ctx, p, x0, x1 - x0, h, dpr)
    }
  }

  redraw(rowSet) {
    for (const r of rowSet) {
      const canvas = this.rows.get(r)
      if (canvas) this.drawRow(r, canvas)
    }
  }

  destroy() {
    this.releaseAll()
    this.el.remove()
  }
}

const heart = typeof Path2D !== 'undefined'
  ? new Path2D('M12 21s-7.5-4.6-10-9.2C.3 8.6 2.1 4.5 6 4.5c2.2 0 3.6 1.2 4.4 2.4h3.2c.8-1.2 2.2-2.4 4.4-2.4 3.9 0 5.7 4.1 4 7.3C19.5 16.4 12 21 12 21z')
  : null

// The heart (favourite) bottom left and a play mark (video) bottom right, like Photos'.
function drawBadges(ctx, p, x, w, h, dpr) {
  const s = Math.min(16, Math.max(11, w / dpr / 9)) * dpr
  const m = 6 * dpr
  ctx.save()
  ctx.shadowColor = 'rgba(0, 0, 0, 0.55)'
  ctx.shadowBlur = 3 * dpr
  ctx.fillStyle = '#fff'
  if (p.favorite && heart) {
    ctx.translate(x + m, h - m - s)
    ctx.scale(s / 24, s / 24)
    ctx.fill(heart)
    ctx.setTransform(1, 0, 0, 1, 0, 0)
  }
  if (p.isVideo) {
    const cx = x + w - m - s / 2
    const cy = h - m - s / 2
    ctx.beginPath()
    ctx.moveTo(cx - s * 0.32, cy - s * 0.4)
    ctx.lineTo(cx + s * 0.42, cy)
    ctx.lineTo(cx - s * 0.32, cy + s * 0.4)
    ctx.closePath()
    ctx.fill()
  }
  ctx.restore()
}

// A critically damped spring from `from` to `to` (no overshoot), starting at `velocity` (progress
// per second): a quick flick lands sooner, a slow release eases in. Same as the app's `Settle`.
class Settle {
  constructor(from, to, velocity, response) {
    this.start = performance.now()
    this.from = from
    this.to = to
    this.velocity = velocity
    this.omega = (2 * Math.PI) / response
  }

  value(now) {
    const t = Math.max(0, (now - this.start) / 1000)
    const d = this.from - this.to
    const decay = Math.exp(-this.omega * t)
    const value = this.to + (d + (this.velocity + this.omega * d) * t) * decay
    const speed = (this.velocity - (this.velocity + this.omega * d) * this.omega * t) * decay
    const done = (Math.abs(value - this.to) < 0.002 && Math.abs(speed) < 0.05) || t > 1.5
    return { value: done ? this.to : value, done }
  }
}

export class PhotoGrid {
  /**
   * scroller: the element that scrolls. opts:
   *  onOpen(index), onMenu(index, { clientX, clientY }), onLevel(level),
   *  onView({ first, last, atNewest }) — the photos on screen, for the title's dates,
   *  chipLabel(item, kind) — 'month' or 'year' label of a photo, for the dense grids' chips.
   */
  constructor(scroller, opts) {
    this.scroller = scroller
    this.opts = opts
    this.items = []
    this.indexById = new Map()
    this.dpr = window.devicePixelRatio || 1
    this.width = scroller.clientWidth
    this.height = scroller.clientHeight
    this.insetTop = 0
    this.insetBottom = 0
    this.scrollTop = 0
    this.session = null // a zoom between two levels in progress
    this.settle = null
    this.pinch = null
    this.focal = { index: 0, v: 0.5 }
    this.finger = { x: 0, y: 0 }
    this.stretch = 1 // past the ends
    this.stretchSpring = null
    this.dirty = new Set() // item ids whose rows need drawing again
    this.pool = []
    this.frame = 0
    this.lastWantKey = ''
    this.lastView = ''

    this.spacer = document.createElement('div')
    this.spacer.className = 'pg-spacer'
    scroller.appendChild(this.spacer)
    this.chipsEl = document.createElement('div')
    this.chipsEl.className = 'pg-chips'
    this.spacer.appendChild(this.chipsEl)
    this.chips = []
    this.pinnedChip = document.createElement('div')
    this.pinnedChip.className = 'pg-chip pg-pinned'
    scroller.parentElement.appendChild(this.pinnedChip)

    this.placeholder = getComputedStyle(scroller).getPropertyValue('--fill-3').trim() || 'rgba(128,128,128,0.15)'
    this.thumbs = new ThumbCache(id => {
      this.dirty.add(id)
      this.schedule()
    })

    const start = opts.level ?? 2
    this.current = new Layer(this, Math.min(start, maxLevelFor(this.width)))
    if (this.current.level !== start) queueMicrotask(() => opts.onLevel?.(this.current.level))

    this.onScroll = () => {
      this.scrollTop = scroller.scrollTop
      this.schedule()
    }
    scroller.addEventListener('scroll', this.onScroll, { passive: true })
    this.bindInput()
    this.resizeObserver = new ResizeObserver(() => this.resize())
    this.resizeObserver.observe(scroller)
  }

  /** The level the grid is at (during a zoom, the one it started from). */
  get level() { return this.session ? this.session.from.level : this.current.level }

  // MARK: data

  /** items: oldest first, each { id, checksum, path, isVideo, favorite, date }. */
  setItems(items, { toNewest = false } = {}) {
    this.stopZoom()
    const keep = !toNewest && this.items.length ? this.anchorAtCenter() : null
    const keepId = keep ? this.items[keep.index]?.id : null
    this.items = items
    this.indexById = new Map(items.map((p, i) => [p.id, i]))
    this.current.relayout()
    this.buildChipGroups()
    this.spacer.style.height = `${this.current.layout.height}px`
    if (toNewest || keepId == null || !this.indexById.has(keepId)) {
      this.setScrollTop(this.current.layout.height)
    } else {
      const L = this.current.layout
      const i = this.indexById.get(keepId)
      this.setScrollTop(L.y(i) + keep.v * L.tile - keep.fy)
    }
    this.schedule()
  }

  /** A photo changed (favourite): draw its row again. */
  refresh(id) {
    this.dirty.add(id)
    this.schedule()
  }

  setInsets(top, bottom) {
    if (top === this.insetTop && bottom === this.insetBottom) return
    this.stopZoom()
    const atBottom = this.isAtNewest()
    this.insetTop = top
    this.insetBottom = bottom
    this.current.relayout()
    this.spacer.style.height = `${this.current.layout.height}px`
    if (atBottom) this.setScrollTop(this.current.layout.height)
    this.schedule()
  }

  resize() {
    const w = this.scroller.clientWidth
    const h = this.scroller.clientHeight
    const dpr = window.devicePixelRatio || 1
    if (w === this.width && h === this.height && dpr === this.dpr) return
    this.stopZoom()
    const keep = this.items.length ? this.anchorAtCenter() : null
    const atBottom = this.isAtNewest()
    this.width = w
    this.height = h
    this.dpr = dpr
    // Narrowed below where 61 columns are offered: drop to 31.
    const max = maxLevelFor(w)
    if (this.current.level > max) {
      this.current.level = max
      this.buildChipGroups()
      this.opts.onLevel?.(max)
    }
    this.current.relayout()
    this.spacer.style.height = `${this.current.layout.height}px`
    if (atBottom || !keep) this.setScrollTop(this.current.layout.height)
    else {
      const L = this.current.layout
      this.setScrollTop(L.y(keep.index) + keep.v * L.tile - keep.fy)
    }
    this.schedule()
  }

  /** Scrolls so a photo is on screen (after closing the viewer on it). */
  reveal(index) {
    const L = this.current.layout
    if (index < 0 || index >= L.count) return
    const y = L.y(index)
    if (y < this.scrollTop + this.insetTop || y + L.tile > this.scrollTop + this.height - this.insetBottom) {
      this.setScrollTop(y - (this.height - L.tile) / 2)
    }
  }

  scrollToNewest() {
    this.setScrollTop(this.current.layout.height)
  }

  isAtNewest() {
    return this.scrollTop + this.height >= this.current.layout.height - 4
  }

  maxScroll(layout) {
    return Math.max(0, layout.height - this.height)
  }

  setScrollTop(y) {
    this.scroller.scrollTop = Math.min(this.maxScroll(this.current.layout), Math.max(0, y))
    this.scrollTop = this.scroller.scrollTop
  }

  anchorAtCenter() {
    const fy = this.insetTop + (this.height - this.insetTop - this.insetBottom) / 2
    const hit = this.current.layout.itemAt(this.width / 2, fy + this.scrollTop)
    return hit ? { ...hit, fy } : null
  }

  // MARK: zoom

  /** Picks the photo under a point on screen as the one the zoom keeps in place. */
  setFocus(fx, fy) {
    this.finger = { x: fx, y: fy }
    const hit = this.current.layout.itemAt(fx, fy + this.scrollTop)
    if (hit) this.focal = hit
  }

  /**
   * Lays the `target` grid out at rest underneath the current one, with the focal photo as close
   * under the fingers as the scroll range allows (so the zoom ends without a correcting jump),
   * and works out the point both grids scale about.
   */
  startSession(target) {
    const from = this.current
    const Lf = from.layout
    if (!Lf.count) return false
    const to = new Layer(this, target)
    this.spacer.insertBefore(to.el, from.el) // the grid being left stays on top
    const Lt = to.layout
    const f = Math.min(this.focal.index, Lt.count - 1)
    const offsetFrom = this.scrollTop
    const offsetTo = Math.min(this.maxScroll(Lt), Math.max(0, Lt.y(f) + Lt.tile * this.focal.v - this.finger.y))
    const ratio = Lt.pitch / Lf.pitch

    // Across: the middle of the grid, so columns leave and arrive evenly on both sides — every
    // level has an odd column count, so the middle column and with it every column of the two
    // grids line up. Down: the height that maps the focal photo's row in the old grid onto its
    // row in the new one, so the rows line up too.
    const a = Lf.y(f) - offsetFrom
    const b = Lt.y(f) - offsetTo
    const pivot = { x: this.width / 2, y: (ratio * a - b) / (ratio - 1) }

    this.current = to
    this.spacer.style.height = `${Math.max(Lt.height, this.height)}px`
    this.setScrollTop(offsetTo)

    from.el.style.transformOrigin = `${pivot.x}px ${pivot.y + offsetFrom}px`
    to.el.style.transformOrigin = `${pivot.x}px ${pivot.y + this.scrollTop}px`

    // Zooming in, the new grid starts out smaller than the screen (its tiles shrunk to the old
    // size), so there's nothing under the old grid's outer rows and columns: fading those too
    // would show the background. Only the part of the old grid over the new one's footprint fades;
    // the rest stays solid and slides off the edges as the grid grows. Zooming out the footprint
    // is the whole screen and the whole layer fades.
    let masked = false
    if (ratio > 1) {
      const x0 = Math.max(0, pivot.x - pivot.x / ratio)
      const x1 = Math.min(this.width, pivot.x + (this.width - pivot.x) / ratio)
      const y0 = Math.max(0, pivot.y - pivot.y / ratio) + offsetFrom
      const y1 = Math.min(this.height, pivot.y + (this.height - pivot.y) / ratio) + offsetFrom
      const H = Lf.height
      const solid = 'linear-gradient(#000, #000)'
      const faded = 'linear-gradient(rgba(0, 0, 0, var(--pg-fade, 1)), rgba(0, 0, 0, var(--pg-fade, 1)))'
      const bands = [
        [solid, 0, 0, this.width, y0],
        [solid, 0, y1, this.width, H - y1],
        [solid, 0, y0, x0, y1 - y0],
        [solid, x1, y0, this.width - x1, y1 - y0],
        [faded, x0, y0, x1 - x0, y1 - y0],
      ].filter(([, , , w, h]) => w > 0 && h > 0)
      const s = from.el.style
      const image = bands.map(b => b[0]).join(', ')
      const size = bands.map(b => `${b[3]}px ${b[4]}px`).join(', ')
      const position = bands.map(b => `${b[1]}px ${b[2]}px`).join(', ')
      s.maskImage = s.webkitMaskImage = image
      s.maskSize = s.webkitMaskSize = size
      s.maskPosition = s.webkitMaskPosition = position
      s.maskRepeat = s.webkitMaskRepeat = 'no-repeat'
      masked = true
    }

    this.session = { target, ratio, pivot, from, to, offsetFrom, offsetTo: this.scrollTop, progress: 0, masked }
    this.setProgress(0)
    return true
  }

  setProgress(progress) {
    const s = this.session
    if (!s) return
    const p = Math.min(1, Math.max(0, progress))
    s.progress = p
    // The old grid grows (or shrinks) by `scale` about the pivot, and the new one with it, from
    // the old tile size (1 / ratio) to its own (1).
    const scale = Math.pow(s.ratio, p)
    const ty = s.offsetTo - s.offsetFrom
    s.from.xf = { oy: s.pivot.y + s.offsetFrom, k: scale, ty }
    s.from.el.style.transform = `translate3d(0, ${ty}px, 0) scale(${scale})`
    s.to.xf = { oy: s.pivot.y + s.offsetTo, k: scale / s.ratio, ty: 0 }
    s.to.el.style.transform = `scale(${scale / s.ratio})`
    if (s.masked) s.from.el.style.setProperty('--pg-fade', String(1 - p))
    else s.from.el.style.opacity = String(1 - p)
    this.schedule()
  }

  /** Lands on the session's target: the new grid is already at rest underneath. */
  commit() {
    const s = this.session
    if (!s) return
    s.from.destroy()
    s.to.clearTransform()
    this.session = null
    this.buildChipGroups()
    this.spacer.style.height = `${this.current.layout.height}px`
    this.opts.onLevel?.(s.target)
    this.schedule()
  }

  /** Back to the grid the session started from, exactly as it was. */
  revert() {
    const s = this.session
    if (!s) return
    s.to.destroy()
    s.from.clearTransform()
    this.current = s.from
    this.session = null
    this.spacer.style.height = `${this.current.layout.height}px`
    this.setScrollTop(s.offsetFrom)
    this.schedule()
  }

  /** Ends whatever zoom is going on at once (data or size changed under it). */
  stopZoom() {
    this.settle = null
    this.pinch = null
    this.unbindPinch()
    if (this.session) {
      if (this.session.progress >= 0.5) this.commit()
      else this.revert()
    }
    this.setStretch(1)
    this.stretchSpring = null
  }

  pinchChanged(scale, raw) {
    // Spreading the fingers zooms in: fewer, bigger columns.
    const target = this.level + (scale >= 1 ? -1 : 1)
    if (target < 0 || target > maxLevelFor(this.width)) {
      this.revert()
      this.setStretch(Math.pow(scale, 0.2))
      return
    }
    if (this.stretch !== 1) this.setStretch(1)
    if (this.session?.target !== target) {
      this.revert()
      if (!this.startSession(target)) return
    }
    const progress = Math.log(scale) / Math.log(this.session.ratio)
    if (progress >= 1) {
      // Reached the next level with the fingers still going: land on it and carry on from there
      // in the same pinch.
      this.setProgress(1)
      this.commit()
      this.pinch.baseline = raw
    } else {
      this.setProgress(Math.max(progress, 0))
    }
  }

  /** velocity: the pinch's scale per second when the fingers lifted. */
  pinchEnded(velocity, scale, cancelled) {
    if (this.stretch !== 1) this.springBackStretch()
    const s = this.session
    if (!s) return
    const zoomingIn = s.ratio > 1
    const flicked = Math.abs(velocity) > 0.6
    const towardTarget = zoomingIn ? velocity > 0 : velocity < 0
    let finish
    if (cancelled) finish = false
    else if (flicked) finish = towardTarget ? s.progress > 0.05 : s.progress > 0.9
    else finish = s.progress > 0.5
    // The pinch's speed in progress per second, so the spring carries on at the fingers' speed.
    const progressVelocity = scale > 0 ? velocity / scale / Math.log(s.ratio) : 0
    this.settleTo(finish ? 1 : 0, cancelled ? 0 : progressVelocity, 0.3)
  }

  settleTo(to, velocity, response) {
    const s = this.session
    if (!s) return
    // Heading the wrong way, the speed is dropped rather than carried into a reversal.
    const v = (to - s.progress) * velocity >= 0 ? Math.min(12, Math.max(-12, velocity)) : 0
    this.settle = new Settle(s.progress, to, v, response)
    this.schedule()
  }

  completeSettleNow() {
    if (!this.settle) return
    const to = this.settle.to
    this.settle = null
    this.setProgress(to)
    if (to === 1) this.commit()
    else this.revert()
  }

  /**
   * Animates to a level — any number of levels away, in one smooth step — about a point of the
   * view (the zoom bar and the keys use the middle, a tap the photo tapped), as if pinched there.
   */
  zoomTo(level, fx = this.width / 2, fy = this.insetTop + (this.height - this.insetTop - this.insetBottom) / 2, response = 0.42) {
    level = Math.min(maxLevelFor(this.width), Math.max(0, level))
    if (this.pinch || !this.items.length) return
    this.completeSettleNow()
    if (level === this.current.level) return
    this.setFocus(fx, fy)
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) {
      if (this.startSession(level)) { this.setProgress(1); this.commit() }
      return
    }
    if (this.startSession(level)) this.settleTo(1, 0, response)
  }

  /** One level in or out (+/− and the keys), counted from where the grid lands if it's still moving. */
  zoomBy(step) {
    if (this.pinch) return
    this.completeSettleNow()
    this.zoomTo(this.current.level + step)
  }

  // Past the densest and the largest levels: the grid stretches a little about the fingers,
  // resisting more the further the pinch goes, and springs back.
  setStretch(amount) {
    this.stretch = amount
    const el = this.current.el
    if (amount === 1) {
      if (!this.session) { el.style.transform = ''; el.style.transformOrigin = '' }
      return
    }
    el.style.transformOrigin = `${this.finger.x}px ${this.finger.y + this.scrollTop}px`
    el.style.transform = `scale(${amount})`
  }

  springBackStretch() {
    // A spring with the app's damping 26 and stiffness 300 (slightly bouncy).
    this.stretchSpring = { x: this.stretch - 1, v: 0, last: performance.now() }
    this.schedule()
  }

  stepStretchSpring(now) {
    const sp = this.stretchSpring
    let dt = Math.min(0.05, (now - sp.last) / 1000)
    sp.last = now
    while (dt > 0) {
      const h = Math.min(dt, 1 / 240)
      sp.v += (-300 * sp.x - 26 * sp.v) * h
      sp.x += sp.v * h
      dt -= h
    }
    if (Math.abs(sp.x) < 0.0005 && Math.abs(sp.v) < 0.005) {
      this.stretchSpring = null
      this.setStretch(1)
      return false
    }
    this.setStretch(1 + sp.x)
    return true
  }

  // MARK: frame

  schedule() {
    if (!this.frame) this.frame = requestAnimationFrame(() => { this.frame = 0; this.render() })
  }

  render() {
    const now = performance.now()
    let again = false

    if (this.settle) {
      const { value, done } = this.settle.value(now)
      this.setProgress(value)
      if (done) {
        const to = this.settle.to
        this.settle = null
        if (to === 1) this.commit()
        else this.revert()
      } else again = true
    }
    if (this.stretchSpring && this.stepStretchSpring(now)) again = true

    const margin = this.session ? 0.25 : 0.6
    for (const layer of [this.session?.from, this.current]) {
      if (!layer) continue
      const [y0, y1] = layer.visibleRange(margin)
      layer.ensureRows(y0, y1)
    }

    if (this.dirty.size) {
      for (const layer of [this.session?.from, this.current]) {
        if (!layer) continue
        const rows = new Set()
        for (const id of this.dirty) {
          const i = this.indexById.get(id)
          if (i !== undefined) rows.add(layer.layout.rowOf(i))
        }
        layer.redraw(rows)
      }
      this.dirty.clear()
    }

    this.placeChips()
    this.reportView()
    this.requestThumbs()
    if (again) this.schedule()
  }

  requestThumbs() {
    const layer = this.current
    const L = layer.layout
    if (!L.rows) return
    const rowAt = y => Math.min(L.rows - 1, Math.max(0, Math.floor((y - L.top) / L.pitch)))
    const [a, b] = layer.visibleRange(0)
    const v0 = rowAt(a)
    const v1 = rowAt(b)
    const wantKey = `${layer.level}|${L.count}|${v0}|${v1}|${this.session ? 1 : 0}`
    if (wantKey === this.lastWantKey) return
    this.lastWantKey = wantKey
    const span = v1 - v0 + 1
    const order = []
    for (let r = v0; r <= v1; r++) order.push(r)
    // Then outwards, nearest first: a screen and a half each way, half a screen in the dense
    // grids, where a screen is already thousands of photos.
    const ahead = L.cols >= 31 ? 0.5 : 1.5
    for (let d = 1; d <= Math.ceil(span * ahead); d++) {
      if (v1 + d < L.rows) order.push(v1 + d)
      if (v0 - d >= 0) order.push(v0 - d)
    }
    const items = []
    const pinned = new Set()
    for (let n = 0; n < order.length; n++) {
      const r = order[n]
      for (let i = L.rowStart(r); i < L.rowEnd(r); i++) {
        items.push(this.items[i])
        if (n < span) pinned.add(this.items[i].id)
      }
    }
    const old = this.session?.from
    if (old) for (const r of old.rows.keys()) {
      for (let i = old.layout.rowStart(r); i < old.layout.rowEnd(r); i++) pinned.add(this.items[i].id)
    }
    this.thumbs.pin(pinned)
    this.thumbs.want(items, layer.size)
  }

  reportView() {
    if (!this.opts.onView) return
    const L = this.current.layout
    if (!L.count) {
      if (this.lastView !== 'empty') { this.lastView = 'empty'; this.opts.onView({ first: -1, last: -1, atNewest: true }) }
      return
    }
    const top = this.scrollTop + this.insetTop
    const bottom = this.scrollTop + this.height - this.insetBottom
    const r0 = Math.min(L.rows - 1, Math.max(0, Math.floor((top - L.top) / L.pitch)))
    const r1 = Math.min(L.rows - 1, Math.max(0, Math.floor((bottom - L.top - 1) / L.pitch)))
    const first = L.rowStart(r0)
    const last = L.rowEnd(r1) - 1
    const atNewest = this.isAtNewest()
    const key = `${first}|${last}|${atNewest}`
    if (key === this.lastView) return
    this.lastView = key
    this.opts.onView({ first, last, atNewest })
  }

  // MARK: chips (15 and 31 columns: the month or year where it begins, the current one pinned)

  buildChipGroups() {
    const cols = LEVELS[this.current.level]
    this.chipKind = cols >= 31 ? 'year' : cols >= 15 ? 'month' : null
    this.groups = []
    if (!this.chipKind || !this.opts.chipLabel) return
    let lastKey = null
    for (let i = 0; i < this.items.length; i++) {
      const d = this.items[i].date
      if (!d) continue // an undated photo joins the group before it
      const key = this.chipKind === 'year' ? d.getFullYear() : d.getFullYear() * 12 + d.getMonth()
      if (key === lastKey) continue
      lastKey = key
      this.groups.push({ index: i, label: this.opts.chipLabel(this.items[i], this.chipKind) })
    }
  }

  placeChips() {
    const show = this.chipKind && !this.session && this.stretch === 1 && this.groups.length
    if (!show) {
      for (const c of this.chips) c.style.display = 'none'
      this.pinnedChip.style.display = 'none'
      return
    }
    const L = this.current.layout
    const pinnedY = this.insetTop + 8 // where the current one stays, in the view
    const chipH = 26
    // The last group starting in each row wins its row's chip.
    const byRow = new Map()
    for (const g of this.groups) byRow.set(L.rowOf(g.index), g)
    const top = this.scrollTop
    const bottom = this.scrollTop + this.height
    let used = 0
    let currentGroup = null
    let nextY = Infinity
    for (const g of this.groups) {
      const y = L.y(g.index) + 6 - this.scrollTop
      if (y <= pinnedY + 0.5) currentGroup = g
      else if (y < nextY && byRow.get(L.rowOf(g.index)) === g) nextY = y
    }
    for (const [r, g] of byRow) {
      const y = L.rowTop(r) + 6
      if (y + chipH < top || y > bottom) continue
      if (y - this.scrollTop <= pinnedY + 0.5) continue // gone under the pinned one
      let chip = this.chips[used]
      if (!chip) {
        chip = document.createElement('div')
        chip.className = 'pg-chip'
        this.chipsEl.appendChild(chip)
        this.chips.push(chip)
      }
      used++
      if (chip.dataset.label !== g.label) { chip.textContent = ''; chip.append(...renderLabel(g.label)); chip.dataset.label = g.label }
      chip.style.display = ''
      chip.style.transform = `translate3d(8px, ${y}px, 0)`
    }
    for (let i = used; i < this.chips.length; i++) this.chips[i].style.display = 'none'

    if (!currentGroup) currentGroup = this.groups[0]
    const push = Math.min(0, nextY - (pinnedY + chipH + 4))
    if (this.pinnedChip.dataset.label !== currentGroup.label) {
      this.pinnedChip.textContent = ''
      this.pinnedChip.append(...renderLabel(currentGroup.label))
      this.pinnedChip.dataset.label = currentGroup.label
    }
    this.pinnedChip.style.display = ''
    this.pinnedChip.style.transform = `translate3d(8px, ${pinnedY + push}px, 0)`
    this.pinnedChip.style.opacity = String(Math.max(0, 1 + push / chipH))
  }

  // MARK: canvases

  takeCanvas(parent) {
    const canvas = this.pool.pop() || document.createElement('canvas')
    canvas.className = 'pg-row'
    parent.appendChild(canvas)
    return canvas
  }

  releaseCanvas(canvas) {
    canvas.remove()
    if (this.pool.length < 120) this.pool.push(canvas)
  }

  // MARK: input

  /** The photo at a point on screen (client coordinates), or -1. */
  indexAt(clientX, clientY) {
    if (this.session || this.stretch !== 1) return -1
    const rect = this.scroller.getBoundingClientRect()
    const x = clientX - rect.left
    const y = clientY - rect.top + this.scrollTop
    const L = this.current.layout
    if (y < L.top || y > L.top + L.rows * L.pitch) return -1
    const r = Math.floor((y - L.top) / L.pitch)
    if (y - L.rowTop(r) > L.tile) return -1
    const c = Math.floor(x / L.pitch)
    if (x - c * L.pitch > L.tile || c >= L.cols) return -1
    const i = L.rowStart(r) + c
    return i < L.rowEnd(r) ? i : -1
  }

  /** Where tapping zooms to instead of opening, like the app: 61 → 31, 31 → 15, 9 or 15 → 5. */
  tapLevel() {
    const cols = LEVELS[this.current.level]
    const target = cols > 31 ? 31 : cols > 15 ? 15 : cols > 5 ? 5 : null
    return target ? LEVELS.indexOf(target) : null
  }

  beginPinch(kind, x, y, raw) {
    this.completeSettleNow()
    this.stretchSpring = null
    if (this.stretch !== 1) this.setStretch(1)
    this.setFocus(x, y)
    this.pinch = { kind, baseline: raw, raw, samples: [{ t: performance.now(), raw }] }
    // The rest of the gesture is followed on the window: its events go to the element it began
    // on, a row of a grid that a level change in the middle of the pinch removes, and a removed
    // element passes nothing on.
    if (kind === 'touch') {
      window.addEventListener('touchmove', this.onTouchMove, { passive: false })
      window.addEventListener('touchend', this.onTouchEnd)
      window.addEventListener('touchcancel', this.onTouchEnd)
    } else if (kind === 'gesture') {
      window.addEventListener('gesturechange', this.onGestureChange)
      window.addEventListener('gestureend', this.onGestureEnd)
    }
  }

  unbindPinch() {
    window.removeEventListener('touchmove', this.onTouchMove)
    window.removeEventListener('touchend', this.onTouchEnd)
    window.removeEventListener('touchcancel', this.onTouchEnd)
    window.removeEventListener('gesturechange', this.onGestureChange)
    window.removeEventListener('gestureend', this.onGestureEnd)
  }

  movePinch(raw) {
    const p = this.pinch
    if (!p) return
    p.raw = raw
    const now = performance.now()
    p.samples.push({ t: now, raw })
    while (p.samples.length > 2 && now - p.samples[0].t > 100) p.samples.shift()
    this.pinchChanged(raw / p.baseline, raw)
  }

  endPinch(cancelled = false) {
    const p = this.pinch
    if (!p) return
    this.pinch = null
    this.unbindPinch()
    const first = p.samples[0]
    const last = p.samples.at(-1)
    const dt = (last.t - first.t) / 1000
    // In scale per second, like a UIPinchGestureRecognizer's: a touch pinch's raw value is the
    // fingers' distance in pixels, so it's taken relative to where this level's pinch began.
    const velocity = dt > 0.008 ? (last.raw - first.raw) / dt / p.baseline : 0
    this.pinchEnded(velocity, p.raw / p.baseline, cancelled)
  }

  bindInput() {
    const el = this.scroller
    const local = (cx, cy) => {
      const r = el.getBoundingClientRect()
      return [cx - r.left, cy - r.top]
    }

    this.onClick = e => {
      const i = this.indexAt(e.clientX, e.clientY)
      if (i < 0) return
      const to = this.tapLevel()
      if (to !== null) this.zoomTo(to, ...local(e.clientX, e.clientY))
      else this.opts.onOpen?.(i)
    }
    this.onContext = e => {
      const i = this.indexAt(e.clientX, e.clientY)
      if (i < 0) return
      e.preventDefault()
      this.opts.onMenu?.(i, { clientX: e.clientX, clientY: e.clientY })
    }

    // Trackpad pinch: Chrome and Firefox send it as wheel events with ctrlKey.
    let wheelTimer = 0
    this.onWheel = e => {
      if (!e.ctrlKey) return
      e.preventDefault()
      if (this.pinch && this.pinch.kind !== 'wheel') return
      if (!this.pinch) this.beginPinch('wheel', ...local(e.clientX, e.clientY), 1)
      const delta = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY
      this.movePinch(this.pinch.raw * Math.exp(-delta * 0.01))
      clearTimeout(wheelTimer)
      wheelTimer = setTimeout(() => this.endPinch(), 120)
    }

    // Safari on a Mac sends gesture events for a trackpad pinch (on iOS touches handle it).
    this.onGestureStart = e => {
      e.preventDefault()
      if (this.pinch) return
      this.beginPinch('gesture', ...local(e.clientX, e.clientY), 1)
    }
    this.onGestureChange = e => {
      e.preventDefault()
      if (this.pinch?.kind === 'gesture') this.movePinch(e.scale)
    }
    this.onGestureEnd = e => {
      e.preventDefault()
      if (this.pinch?.kind === 'gesture') this.endPinch()
    }

    // Touch screens: two fingers.
    const two = e => {
      const [a, b] = [e.touches[0], e.touches[1]]
      const [ax, ay] = local(a.clientX, a.clientY)
      const [bx, by] = local(b.clientX, b.clientY)
      return { x: (ax + bx) / 2, y: (ay + by) / 2, d: Math.hypot(ax - bx, ay - by) || 1 }
    }
    this.onTouchStart = e => {
      if (e.touches.length !== 2 || this.pinch) return
      const t = two(e)
      this.beginPinch('touch', t.x, t.y, t.d)
      e.preventDefault()
    }
    this.onTouchMove = e => {
      if (this.pinch?.kind !== 'touch' || e.touches.length < 2) return
      e.preventDefault()
      this.movePinch(two(e).d)
    }
    this.onTouchEnd = e => {
      if (this.pinch?.kind !== 'touch' || e.touches.length >= 2) return
      this.endPinch(e.type === 'touchcancel')
    }

    el.addEventListener('click', this.onClick)
    el.addEventListener('contextmenu', this.onContext)
    el.addEventListener('wheel', this.onWheel, { passive: false })
    el.addEventListener('gesturestart', this.onGestureStart)
    el.addEventListener('touchstart', this.onTouchStart, { passive: false })
  }

  destroy() {
    cancelAnimationFrame(this.frame)
    this.resizeObserver.disconnect()
    const el = this.scroller
    el.removeEventListener('scroll', this.onScroll)
    el.removeEventListener('click', this.onClick)
    el.removeEventListener('contextmenu', this.onContext)
    el.removeEventListener('wheel', this.onWheel)
    el.removeEventListener('gesturestart', this.onGestureStart)
    el.removeEventListener('touchstart', this.onTouchStart)
    this.unbindPinch()
    this.session?.from.destroy()
    this.current.destroy()
    this.spacer.remove()
    this.pinnedChip.remove()
    this.thumbs.destroy()
  }
}

// "Aug 2026" with the month bold, as in Photos; a year alone is bold.
function renderLabel(label) {
  const space = label.lastIndexOf(' ')
  const strong = document.createElement('b')
  if (space < 0) { strong.textContent = label; return [strong] }
  strong.textContent = label.slice(0, space)
  return [strong, document.createTextNode(label.slice(space))]
}
