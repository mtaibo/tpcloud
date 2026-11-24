// The take: one pile of points. The file → everything it carries → the machine at home → the devices
// reaching it → the race home vs internet (16 ms / 180 ms, measured from a Mac) → the file again, which
// turns to dust as the download comes up. Scroll drives the moves; the packets run on real time.
(() => {
  const { smooth, mid, reduced, take, caption } = TP
  const cv = document.getElementById('pts'), cx = cv.getContext('2d')
  const label = document.getElementById('label')
  const caps = [...document.querySelectorAll('.cap')]
  const track = document.getElementById('track')
  const LAST = 5
  const race = document.getElementById('race'), ta = document.getElementById('ta'), tb = document.getElementById('tb')
  let W, H, dpr, C, F, N, seed, scenes, sig, paths, packet, R

  // Draws a scene offscreen in an F×F box and returns its lit pixels, as points around the centre.
  function sample(draw) {
    const S = Math.ceil(F), off = new OffscreenCanvas(S, S), c = off.getContext('2d')
    c.strokeStyle = c.fillStyle = '#fff'; c.lineWidth = Math.max(2, F / 260); c.lineJoin = c.lineCap = 'round'
    draw(c, S)
    const { data } = c.getImageData(0, 0, S, S), lit = []
    for (let y = 0; y < S; y += 1) for (let x = 0; x < S; x += 1) if (data[(x + y * S) * 4 + 3] > 120) lit.push(x - S / 2, y - S / 2)
    return lit
  }
  const rr = (c, x, y, w, h, r) => { c.beginPath(); c.roundRect(x, y, w, h, r); c.stroke() }
  const pick = (lit, i) => { const j = (Math.floor(seed[i] * 9973 * lit.length / 2) % (lit.length / 2)) * 2; return [lit[j], lit[j + 1]] }

  function file(c, S) { // a document with its corner folded, and lines of contents
    const w = S * .44, h = S * .58, x = (S - w) / 2, y = (S - h) / 2 - S * .04, f = w * .26
    c.beginPath(); c.moveTo(x, y); c.lineTo(x + w - f, y); c.lineTo(x + w, y + f); c.lineTo(x + w, y + h); c.lineTo(x, y + h); c.closePath(); c.stroke()
    c.beginPath(); c.moveTo(x + w - f, y); c.lineTo(x + w - f, y + f); c.lineTo(x + w, y + f); c.stroke()
    for (let k = 0; k < 6; k++) { const ly = y + h * (.36 + k * .1); c.beginPath(); c.moveTo(x + w * .16, ly); c.lineTo(x + w * (k % 3 === 2 ? .56 : .84), ly); c.stroke() }
  }
  function machine(c, S, scale = 1, ox = 0, oy = 0) { // a small box with vents and two lights
    const w = S * .66 * scale, h = S * .22 * scale, x = S / 2 - w / 2 + ox, y = S / 2 - h / 2 + oy
    rr(c, x, y, w, h, h * .18)
    for (let k = 0; k < 7; k++) { const vx = x + w * (.1 + k * .045); c.beginPath(); c.moveTo(vx, y + h * .3); c.lineTo(vx, y + h * .7); c.stroke() }
    return { lights: [[x + w * .78 - S / 2, y + h * .5 - S / 2], [x + w * .86 - S / 2, y + h * .5 - S / 2]], centre: [x + w / 2 - S / 2, y + h / 2 - S / 2] }
  }
  function devices(c, S) {
    const m = machine(c, S, .5, 0, S * .02)
    // a phone, left
    const pw = S * .13, ph = S * .27, px = S * .08, py = S * .5 - ph / 2 - S * .17
    rr(c, px, py, pw, ph, pw * .22); rr(c, px + pw * .36, py + ph * .04, pw * .28, ph * .025, 2)
    // a laptop, right
    const lw = S * .3, lh = S * .19, lx = S * .64, ly = S * .2
    rr(c, lx, ly, lw, lh, 6); c.beginPath(); c.moveTo(lx - lw * .08, ly + lh + 8); c.lineTo(lx + lw * 1.08, ly + lh + 8); c.stroke()
    // a window, bottom right (another computer)
    const ww = S * .28, wh = S * .18, wx = S * .6, wy = S * .7
    rr(c, wx, wy, ww, wh, 6); c.beginPath(); c.moveTo(wx, wy + wh * .2); c.lineTo(wx + ww, wy + wh * .2); c.stroke()
    const to = (x, y) => [x - S / 2, y - S / 2]
    return { m, ends: [to(px + pw, py + ph / 2), to(lx, ly + lh / 2), to(wx, wy + wh / 2)] }
  }

  function layout() {
    dpr = Math.min(2, devicePixelRatio || 1)
    W = innerWidth; H = Math.max(innerHeight, cv.clientHeight)
    cv.width = W * dpr; cv.height = H * dpr
    const phone = W < 760
    C = phone ? [W / 2, H * .34] : [W * .64, H * .5]
    F = phone ? Math.min(W * .92, H * .5) : Math.min(W * .56, H * .82)
    N = phone ? 1500 : 2600
    if (!seed || seed.length !== N) { seed = new Float32Array(N); for (let i = 0; i < N; i++) seed[i] = Math.random() }

    const litFile = sample(file), litMachine = sample(c => machine(c, F))
    let net; const litNet = sample((c, S) => { net = devices(c, S) })
    const lightsM = machine(new OffscreenCanvas(1, 1).getContext('2d'), F).lights
    scenes = [new Float32Array(N * 2), new Float32Array(N * 2), new Float32Array(N * 2), new Float32Array(N * 2), new Float32Array(N * 2)]
    sig = [new Uint8Array(N), new Uint8Array(N), new Uint8Array(N), new Uint8Array(N), new Uint8Array(N)]
    packet = new Int8Array(N).fill(-1)
    for (let i = 0; i < N; i++) {
      const [fx, fy] = pick(litFile, i); scenes[0][i * 2] = scenes[4][i * 2] = fx; scenes[0][i * 2 + 1] = scenes[4][i * 2 + 1] = fy
      // everything inside: a wide cloud, denser in the middle
      const a = seed[i] * Math.PI * 2 * 37, r = Math.sqrt((seed[i] * 7919) % 1) * F * .62
      scenes[1][i * 2] = Math.cos(a) * r * 1.25; scenes[1][i * 2 + 1] = Math.sin(a) * r * .8
      let [mx, my] = pick(litMachine, i)
      if (i % 40 === 0) { const L = lightsM[(i / 40) % 2]; mx = L[0] + (seed[i] - .5) * 6; my = L[1] + ((seed[i] * 31) % 1 - .5) * 6; sig[2][i] = 1 }
      scenes[2][i * 2] = mx; scenes[2][i * 2 + 1] = my
      let [nx, ny] = pick(litNet, i)
      if (i % 9 === 0) { packet[i] = (i / 9) % 3; sig[3][i] = 1 }
      else if (i % 40 === 0) { const L = net.m.lights[(i / 40) % 2]; nx = L[0]; ny = L[1]; sig[3][i] = 1 }
      scenes[3][i * 2] = nx; scenes[3][i * 2 + 1] = ny
    }
    paths = net.ends.map(e => [net.m.centre, e])
    // The race: your phone and home side by side, the internet far above them.
    const P = [-F * .22, F * .2], Hm = [F * .22, F * .2], Nt = [0, -F * .34]
    R = { P, Hm, Nt, ctl: [2 * Nt[0] - (P[0] + Hm[0]) / 2, 2 * Nt[1] - (P[1] + Hm[1]) / 2] }
    const litRace = sample((c, S) => {
      const o = S / 2, q = ([x, y]) => [x + o, y + o]
      const [px, py] = q(P), [hx, hy] = q(Hm), [cx2, cy2] = q(R.ctl)
      c.beginPath(); c.moveTo(px, py); c.quadraticCurveTo(cx2, cy2, hx, hy); c.stroke()
      c.beginPath(); c.moveTo(px, py); c.lineTo(hx, hy); c.stroke()
      c.beginPath(); c.arc(px, py, S * .022, 0, 7); c.fill()
      c.beginPath(); c.arc(hx, hy, S * .03, 0, 7); c.stroke()
    })
    scenes.splice(4, 0, new Float32Array(N * 2)); sig.splice(4, 0, new Uint8Array(N))
    for (let i = 0; i < N; i++) {
      const [rx, ry] = pick(litRace, i); scenes[4][i * 2] = rx; scenes[4][i * 2 + 1] = ry
      if (i % 24 === 0) sig[4][i] = 1           // the packet going straight home
      else if (i % 24 === 1) sig[4][i] = 2      // the one going around the internet
      else if (i % 30 === 0) { scenes[4][i * 2] = Hm[0] + (seed[i] - .5) * F * .04; scenes[4][i * 2 + 1] = Hm[1] + ((seed[i] * 31) % 1 - .5) * F * .04; sig[4][i] = 1 }
    }
    label.style.left = C[0] + 'px'; label.style.top = (C[1] + F * .27) + 'px'
  }

  // Where point i is in scene k right now (the cloud drifts, the packets travel: real time).
  function at(k, i, t) {
    let x = scenes[k][i * 2], y = scenes[k][i * 2 + 1]
    if (k === 1 && !reduced) { x += Math.sin(t * .4 + seed[i] * 40) * 10; y += Math.cos(t * .33 + seed[i] * 23) * 8 }
    if (k === 3 && packet[i] >= 0) {
      const [[ax, ay], [bx, by]] = paths[packet[i]]
      let p = ((reduced ? 0 : t * .45) + seed[i] * 3) % 2; p = p > 1 ? 2 - p : p
      x = mid(ax, bx, p); y = mid(ay, by, p)
    }
    if (k === 4 && sig[4][i] && i % 24 < 2) {
      // Real time, slowed down 20×: 16 ms → 0.32 s, 180 ms → 3.6 s, then a breath.
      const cyc = reduced ? 3.6 : t % 4.6, jx = (seed[i] - .5) * F * .025, jy = ((seed[i] * 17) % 1 - .5) * F * .025
      if (i % 24 === 0) { const f = Math.min(1, cyc / .32); x = mid(R.P[0], R.Hm[0], f) + jx; y = mid(R.P[1], R.Hm[1], f) + jy }
      else { const f = Math.min(1, cyc / 3.6), a = [mid(R.P[0], R.ctl[0], f), mid(R.P[1], R.ctl[1], f)], b = [mid(R.ctl[0], R.Hm[0], f), mid(R.ctl[1], R.Hm[1], f)]; x = mid(a[0], b[0], f) + jx; y = mid(a[1], b[1], f) + jy }
    }
    return [x, y]
  }
  function raceText(s, t) {
    const o = 1 - smooth(.08, .3, Math.abs(s - 4))
    race.style.opacity = o; race.style.visibility = o < .02 ? 'hidden' : 'visible'
    if (o < .02) return
    const cyc = reduced ? 3.6 : t % 4.6
    ta.textContent = `${Math.round(Math.min(1, cyc / .32) * 16)} ms`; tb.textContent = `${Math.round(Math.min(1, cyc / 3.6) * 180)} ms`
    if (W < 760) { race.style.left = '16px'; race.style.right = '16px'; race.style.top = (C[1] + F * .36) + 'px' }
    else { race.style.left = Math.max(16, W * .06) + 'px'; race.style.right = ''; race.style.top = (H * .16) + 'px' }
    // The three places, named.
    cx.font = '500 14px -apple-system, system-ui, sans-serif'; cx.textAlign = 'center'; cx.fillStyle = `rgba(140,138,133,${o})`
    cx.fillText('your phone', C[0] + R.P[0], C[1] + R.P[1] + 30)
    cx.fillText('home', C[0] + R.Hm[0], C[1] + R.Hm[1] + 32)
    cx.fillText('the internet', C[0] + R.Nt[0], C[1] + R.Nt[1] - 18)
  }

  let ready = false
  addEventListener('resize', () => { layout() })
  layout(); ready = true

  const dl = () => document.getElementById('download')
  take(track, LAST, (s, rs, t) => {
    if (!ready) return
    // Leaving for the download: as its section comes up, the file comes apart into dust that rises
    // and fades (scrubbed by the scroll, so it plays backwards too).
    const sec = dl(), p = sec ? TP.clamp01(1 - sec.getBoundingClientRect().top / innerHeight) : 0
    const gone = smooth(0, .75, p)
    cv.parentNode.style.visibility = gone > .995 ? 'hidden' : 'visible'
    caps.forEach(el => { const o = caption(el, s, +el.dataset.at) * (1 - smooth(0, .45, p)); el.style.opacity = o; el.style.visibility = o < .02 ? 'hidden' : 'visible' })
    label.style.opacity = Math.max(1 - smooth(.04, .25, s), smooth(LAST - .25, LAST - .04, s)) * (1 - smooth(0, .3, p))
    const k = Math.min(LAST - 1, Math.floor(s)), u = s - k
    cx.setTransform(dpr, 0, 0, dpr, 0, 0); cx.clearRect(0, 0, W, H); cx.globalAlpha = 1 - gone
    const size = W < 760 ? 1.5 : 1.7
    for (let i = 0; i < N; i++) {
      const e = smooth(0, 1, (u - seed[i] * .34) / .66)
      const [x0, y0] = at(k, i, t), [x1, y1] = at(k + 1, i, t)
      const lift = Math.sin(e * Math.PI) * F * .09 * (seed[i] - .5)
      let x = C[0] + mid(x0, x1, e) + lift * .6, y = C[1] + mid(y0, y1, e) - lift
      if (p > 0) { const d = Math.pow(p, 1.3); x += (seed[i] - .5) * F * .5 * d; y -= (.3 + seed[i] * 1.2) * H * .45 * d }
      const g = e < .5 ? sig[k][i] : sig[k + 1][i]
      cx.fillStyle = g === 1 ? '#9cc9ff' : g === 2 ? '#edebe6' : (seed[i] > .5 ? 'rgba(237,235,230,.9)' : 'rgba(237,235,230,.55)')
      cx.fillRect(x, y, g ? size * 1.8 : size, g ? size * 1.8 : size)
    }
    cx.globalAlpha = 1
    raceText(s, t)
  })
})()
