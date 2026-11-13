import { ThumbCache, sizeFor } from './thumbs.js'

// The gallery's grid, built like the iPhone app's: no element per photo (11,000 of them made
// the browser stutter), but one <canvas> per row on screen, drawn from small decoded thumbnails
// and recycled as rows scroll in and out. The page scrolls natively, so the browser's compositor
// moves it at the screen's rate whatever the main thread is doing.
//
// Same densities as the app (3, 5, 9, 15 and 31 columns, all odd), newest photo at the bottom,
// rows cut from the newest end so the partial row is the oldest one, at the top. Zooming (a pinch
// on a trackpad or a touch screen, the zoom bar, the keyboard) keeps the photo under the fingers
// where it is: the grid scales live, switches to the nearest density as it goes, cross-fades the
// old layout out, and settles the photo into its new column when you let go.

export const LEVELS = [3, 5, 9, 15, 31]
const FADE_MS = 220
const SETTLE_MS = 340

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
    const u = Math.min(1, Math.max(0, (x - this.x(i)) / this.tile))
    const v = Math.min(1, Math.max(0, (y - this.rowTop(r)) / this.tile))
    return { index: i, u, v }
  }
}

class Layer {
  constructor(grid, level) {
    this.grid = grid
    this.level = level
    this.el = document.createElement('div')
    this.el.className = 'pg-layer'
    this.rows = new Map() // row -> canvas
    this.opacity = 1
    this.fadeStart = 0
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

  anchorPoint(anchor) {
    const L = this.layout
    return { x: L.x(anchor.index) + anchor.u * L.tile, y: L.y(anchor.index) + anchor.v * L.tile }
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
    this.zoom = null // { anchor, fx, fy, T } while zooming
    this.anim = null
    this.pinch = null
    this.outgoing = null
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

    this.current = new Layer(this, opts.level ?? 2)

    this.onScroll = () => {
      this.scrollTop = scroller.scrollTop
      this.schedule()
    }
    scroller.addEventListener('scroll', this.onScroll, { passive: true })
    this.bindInput()
    this.resizeObserver = new ResizeObserver(() => this.resize())
    this.resizeObserver.observe(scroller)
  }

  get level() { return this.current.level }

  // MARK: data

  /** items: oldest first, each { id, checksum, path, isVideo, favorite, date }. */
  setItems(items, { toNewest = false } = {}) {
    const keep = !toNewest && this.items.length ? this.anchorAtCenter() : null
    const keepId = keep ? this.items[keep.anchor.index]?.id : null
    this.items = items
    this.indexById = new Map(items.map((p, i) => [p.id, i]))
    this.dropOutgoing()
    this.zoom = null
    this.anim = null
    this.current.relayout()
    this.buildChipGroups()
    this.spacer.style.height = `${this.current.layout.height}px`
    if (toNewest || keepId == null || !this.indexById.has(keepId)) {
      this.setScrollTop(this.current.layout.height)
    } else {
      const A = this.current.anchorPoint({ ...keep.anchor, index: this.indexById.get(keepId) })
      this.setScrollTop(A.y - keep.fy)
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
    const keep = this.items.length ? this.anchorAtCenter() : null
    const atBottom = this.isAtNewest()
    this.width = w
    this.height = h
    this.dpr = dpr
    this.dropOutgoing()
    this.zoom = null
    this.anim = null
    this.current.relayout()
    this.spacer.style.height = `${this.current.layout.height}px`
    if (atBottom || !keep) this.setScrollTop(this.current.layout.height)
    else this.setScrollTop(this.current.anchorPoint(keep.anchor).y - keep.fy)
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

  setScrollTop(y) {
    const max = Math.max(0, this.current.layout.height - this.height)
    this.scroller.scrollTop = Math.min(max, Math.max(0, y))
    this.scrollTop = this.scroller.scrollTop
  }

  anchorAtCenter() {
    const fx = this.width / 2
    const fy = this.insetTop + (this.height - this.insetTop - this.insetBottom) / 2
    const hit = this.current.layout.itemAt(fx, fy + this.scrollTop)
    return hit ? { anchor: hit, fx, fy } : null
  }

  // MARK: zoom

  levelTile(level) {
    const cols = LEVELS[level]
    return (this.width - gapFor(cols) * (cols - 1)) / cols
  }

  /** The point of the layout under a point of the view, taking a zoom in progress into account. */
  layoutPoint(layer, vx, vy) {
    if (!this.zoom) return { x: vx, y: vy + this.scrollTop }
    const { anchor, fx, fy, T } = this.zoom
    const k = T / layer.layout.tile
    const A = layer.anchorPoint(anchor)
    return { x: A.x + (vx - fx) / k, y: A.y + (vy - fy) / k }
  }

  beginZoom(fx, fy) {
    const T = this.zoom ? this.zoom.T : this.current.layout.tile
    const p = this.layoutPoint(this.current, fx, fy)
    this.dropOutgoing()
    const hit = this.current.layout.itemAt(p.x, p.y)
    if (!hit) return false
    // Re-anchored on the new point: the layer is put where it already is under the new anchor.
    this.zoom = { anchor: hit, fx, fy, T }
    this.anim = null
    return true
  }

  /** Live zoom: the photo under (fx, fy) at T px. Switches density when another level fits better. */
  updateZoom(T, fx, fy) {
    const z = this.zoom
    if (!z) return
    const minT = this.levelTile(LEVELS.length - 1) * 0.6
    const maxT = this.levelTile(0) * 1.5
    z.T = Math.min(maxT, Math.max(minT, T))
    z.fx = fx
    z.fy = fy
    const best = this.nearestLevel(z.T)
    if (best !== this.current.level) this.switchLevel(best)
    this.schedule()
  }

  nearestLevel(T) {
    let best = 0
    let bestDiff = Infinity
    for (let l = 0; l < LEVELS.length; l++) {
      const d = Math.abs(Math.log(T / this.levelTile(l)))
      if (d < bestDiff) { best = l; bestDiff = d }
    }
    return best
  }

  switchLevel(level) {
    const z = this.zoom
    this.dropOutgoing()
    this.outgoing = this.current
    this.outgoing.fadeStart = performance.now()
    this.current = new Layer(this, level)
    this.buildChipGroups()
    this.spacer.style.height = `${Math.max(this.current.layout.height, this.height)}px`
    const A = this.current.anchorPoint(z.anchor)
    this.setScrollTop(A.y - z.fy)
    this.opts.onLevel?.(level)
  }

  /** Lets go: settles the photo into its column at the new density's size. */
  endZoom() {
    const z = this.zoom
    if (!z) return
    this.anim = { start: performance.now(), from: { T: z.T, fx: z.fx, fy: z.fy } }
    this.schedule()
  }

  /** Zooms one way or the other by itself (zoom bar, keyboard, tap), about a point of the view. */
  zoomTo(level, fx = this.width / 2, fy = this.insetTop + (this.height - this.insetTop - this.insetBottom) / 2) {
    level = Math.min(LEVELS.length - 1, Math.max(0, level))
    if (level === this.current.level || !this.items.length) return
    if (!this.beginZoom(fx, fy)) return
    this.switchLevel(level)
    this.endZoom()
  }

  dropOutgoing() {
    if (!this.outgoing) return
    this.outgoing.destroy()
    this.outgoing = null
  }

  // MARK: frame

  schedule() {
    if (!this.frame) this.frame = requestAnimationFrame(() => { this.frame = 0; this.render() })
  }

  render() {
    const now = performance.now()
    let again = false
    const z = this.zoom

    if (z && this.anim) {
      const t = Math.min(1, (now - this.anim.start) / SETTLE_MS)
      const e = 1 - Math.pow(1 - t, 3)
      const A = this.current.anchorPoint(z.anchor)
      const to = { T: this.current.layout.tile, fx: A.x, fy: A.y - this.scrollTop }
      const from = this.anim.from
      z.T = from.T + (to.T - from.T) * e
      z.fx = from.fx + (to.fx - from.fx) * e
      z.fy = from.fy + (to.fy - from.fy) * e
      if (t >= 1) { this.zoom = null; this.anim = null }
      else again = true
    }

    if (this.outgoing) {
      const t = (now - this.outgoing.fadeStart) / FADE_MS
      if (t >= 1 && !this.pinch) this.dropOutgoing()
      else {
        this.outgoing.opacity = Math.max(0, 1 - t)
        again = true
      }
    }
    if (!this.zoom && !this.outgoing) this.spacer.style.height = `${this.current.layout.height}px`

    for (const layer of [this.outgoing, this.current]) {
      if (!layer) continue
      this.placeLayer(layer)
    }

    if (this.dirty.size) {
      for (const layer of [this.outgoing, this.current]) {
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

  placeLayer(layer) {
    const L = layer.layout
    let y0, y1
    if (this.zoom) {
      const { anchor, fx, fy, T } = this.zoom
      const k = T / L.tile
      const A = layer.anchorPoint(anchor)
      layer.el.style.transformOrigin = `${A.x}px ${A.y}px`
      layer.el.style.transform = `translate3d(${fx - A.x}px, ${fy + this.scrollTop - A.y}px, 0) scale(${k})`
      y0 = A.y + (0 - fy) / k
      y1 = A.y + (this.height - fy) / k
      const pad = (y1 - y0) * 0.15
      y0 -= pad
      y1 += pad
    } else {
      layer.el.style.transform = ''
      y0 = this.scrollTop - this.height * 0.6
      y1 = this.scrollTop + this.height * 1.6
    }
    layer.el.style.opacity = layer === this.outgoing ? String(layer.opacity) : ''
    layer.ensureRows(y0, y1)
  }

  requestThumbs() {
    const layer = this.current
    const L = layer.layout
    if (!L.rows) return
    const rowAt = y => Math.min(L.rows - 1, Math.max(0, Math.floor((y - L.top) / L.pitch)))
    let v0, v1
    if (this.zoom) {
      const a = this.layoutPoint(layer, 0, 0)
      const b = this.layoutPoint(layer, 0, this.height)
      v0 = rowAt(a.y); v1 = rowAt(b.y)
    } else {
      v0 = rowAt(this.scrollTop); v1 = rowAt(this.scrollTop + this.height)
    }
    const wantKey = `${layer.level}|${L.count}|${v0}|${v1}|${this.outgoing ? 1 : 0}`
    if (wantKey === this.lastWantKey) return
    this.lastWantKey = wantKey
    const span = v1 - v0 + 1
    const order = []
    for (let r = v0; r <= v1; r++) order.push(r)
    // Then outwards, a screen and a half each way, nearest first.
    for (let d = 1; d <= Math.ceil(span * 1.5); d++) {
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
    if (this.outgoing) for (const r of this.outgoing.rows.keys()) {
      for (let i = this.outgoing.layout.rowStart(r); i < this.outgoing.layout.rowEnd(r); i++) pinned.add(this.items[i].id)
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
    const show = this.chipKind && !this.zoom && this.groups.length
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
    if (this.zoom) return -1
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

  /** Where tapping zooms to instead of opening, like the app: 31 → 15, 9 or 15 → 5. */
  tapLevel() {
    const cols = LEVELS[this.current.level]
    const target = cols > 15 ? 15 : cols > 5 ? 5 : null
    return target ? LEVELS.indexOf(target) : null
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
      const [x, y] = local(e.clientX, e.clientY)
      if (!this.pinch) {
        if (!this.beginZoom(x, y)) return
        this.pinch = { kind: 'wheel', T0: this.zoom.T, scale: 1 }
      }
      const delta = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY
      this.pinch.scale *= Math.exp(-delta * 0.012)
      this.updateZoom(this.pinch.T0 * this.pinch.scale, x, y)
      clearTimeout(wheelTimer)
      wheelTimer = setTimeout(() => { this.pinch = null; this.endZoom() }, 140)
    }

    // Safari on a Mac sends gesture events for a trackpad pinch (on iOS touches handle it).
    this.onGestureStart = e => {
      e.preventDefault()
      if (this.pinch) return
      const [x, y] = local(e.clientX, e.clientY)
      if (!this.beginZoom(x, y)) return
      this.pinch = { kind: 'gesture', T0: this.zoom.T, x, y }
    }
    this.onGestureChange = e => {
      e.preventDefault()
      if (this.pinch?.kind !== 'gesture') return
      this.updateZoom(this.pinch.T0 * e.scale, this.pinch.x, this.pinch.y)
    }
    this.onGestureEnd = e => {
      e.preventDefault()
      if (this.pinch?.kind !== 'gesture') return
      this.pinch = null
      this.endZoom()
    }

    // Touch screens: two fingers.
    const two = e => {
      const [a, b] = [e.touches[0], e.touches[1]]
      const [ax, ay] = local(a.clientX, a.clientY)
      const [bx, by] = local(b.clientX, b.clientY)
      return { x: (ax + bx) / 2, y: (ay + by) / 2, d: Math.hypot(ax - bx, ay - by) || 1 }
    }
    this.onTouchStart = e => {
      if (e.touches.length !== 2) return
      const t = two(e)
      if (!this.beginZoom(t.x, t.y)) return
      this.pinch = { kind: 'touch', T0: this.zoom.T, d0: t.d }
      e.preventDefault()
    }
    this.onTouchMove = e => {
      if (this.pinch?.kind !== 'touch' || e.touches.length < 2) return
      e.preventDefault()
      const t = two(e)
      this.updateZoom(this.pinch.T0 * (t.d / this.pinch.d0), t.x, t.y)
    }
    this.onTouchEnd = e => {
      if (this.pinch?.kind !== 'touch' || e.touches.length >= 2) return
      this.pinch = null
      this.endZoom()
    }

    el.addEventListener('click', this.onClick)
    el.addEventListener('contextmenu', this.onContext)
    el.addEventListener('wheel', this.onWheel, { passive: false })
    el.addEventListener('gesturestart', this.onGestureStart)
    el.addEventListener('gesturechange', this.onGestureChange)
    el.addEventListener('gestureend', this.onGestureEnd)
    el.addEventListener('touchstart', this.onTouchStart, { passive: false })
    el.addEventListener('touchmove', this.onTouchMove, { passive: false })
    el.addEventListener('touchend', this.onTouchEnd)
    el.addEventListener('touchcancel', this.onTouchEnd)
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
    el.removeEventListener('gesturechange', this.onGestureChange)
    el.removeEventListener('gestureend', this.onGestureEnd)
    el.removeEventListener('touchstart', this.onTouchStart)
    el.removeEventListener('touchmove', this.onTouchMove)
    el.removeEventListener('touchend', this.onTouchEnd)
    el.removeEventListener('touchcancel', this.onTouchEnd)
    this.dropOutgoing()
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
