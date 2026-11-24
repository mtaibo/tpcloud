// The download section: this device's line first, the rest one tap away. Also the scroll-to-moments
// clock (TP.take) and the helpers the take in scene.js runs on.
window.TP = (() => {
  const clamp01 = v => Math.max(0, Math.min(1, v))
  const smooth = (a, b, v) => { const t = clamp01((v - a) / (b - a)); return t * t * (3 - 2 * t) }
  const mid = (a, b, t) => a + (b - a) * t
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches
  const make = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text != null) e.textContent = text; return e }

  // This device.
  const ua = navigator.userAgent
  const os = /iPhone|iPad/.test(ua) || (/Mac/.test(ua) && navigator.maxTouchPoints > 1) ? 'iphone'
    : /Android/.test(ua) ? 'other' : /Mac/.test(ua) ? 'mac' : /Windows/.test(ua) ? 'other' : /Linux|X11|CrOS/.test(ua) ? 'linux' : 'mac'
  let arch = /aarch64|arm64|armv8/i.test(ua) ? 'arm64' : 'amd64'

  const GET = 'curl -fsSL https://tpcloud.migueltaibo.com/get | sh'
  const INSTALL = 'curl -fsSL https://tpcloud.migueltaibo.com/install | sh'
  const cmd = text => {
    const box = make('div', 'cmd'), code = make('code'), btn = make('button', 'copy', 'Copy')
    code.append(make('span', 'p', '$'), document.createTextNode(text)); btn.type = 'button'
    btn.addEventListener('click', async () => {
      try { await navigator.clipboard.writeText(text) } catch { const r = document.createRange(); r.selectNodeContents(code); getSelection().removeAllRanges(); getSelection().addRange(r) }
      btn.textContent = 'Copied'; btn.classList.add('done'); setTimeout(() => { btn.textContent = 'Copy'; btn.classList.remove('done') }, 1600)
    })
    box.append(code, btn); return box
  }
  const para = html => { const p = make('p'); for (const part of html) p.append(typeof part === 'string' ? part : make('b', null, part[0])); return p }
  const link = (href, label, small) => { const a = make('a', null, label); a.href = href; if (small) a.append(make('small', null, small)); return a }

  const PANES = {
    mac: ['Mac', () => [make('h3', null, 'On your Mac'), cmd(GET),
      para(['The ', ['tpcloud'], ' command, to sign in and run your cloud from the terminal. The app is on TestFlight.']),
      Object.assign(make('div', 'files'), {}).appendChild(link('/download/cli/macos', 'Or the file', '4 MB')).parentNode]],
    linux: ['Linux', () => [make('h3', null, 'On your Linux computer'), cmd(GET),
      para(['The TPCloud app and the ', ['tpcloud'], ' command, in one go.']),
      (() => { const f = make('div', 'files'); f.append(link(`/download/app/linux/${arch}`, 'The app', `${arch} · ${arch === 'arm64' ? 18 : 20} MB`), link(`/download/cli/linux/${arch}`, 'tpcloud', `${arch} · 2 MB`)); return f })()]],
    iphone: ['iPhone and iPad', () => [make('h3', null, 'On your iPhone'), para(['Files, photos backed up by themselves, your home. It finds your cloud on the Wi-Fi.']), make('span', 'tag', 'On TestFlight')]],
    other: ['Windows and Android', () => [make('h3', null, 'Anywhere else'), para(['Open your cloud in the browser: ', ['tpcloud.local'], ' at home, your own address outside.'])]],
    server: ['Your cloud', () => [make('h3', null, 'The machine that is your cloud'), cmd(INSTALL),
      para(['On a Raspberry Pi or a PC with Linux. It asks a few questions, there or from your phone.']),
      (() => { const f = make('div', 'files'); f.append(link('/download/linux/arm64', 'Raspberry Pi', 'arm64 · 68 MB'), link('/download/linux/amd64', 'PC', 'amd64 · 76 MB')); return f })()]],
  }

  function download(root) {
    root.classList.add('dl'); root.id = 'download'
    root.append(make('h2', null, 'Get TPCloud.'))
    const devs = make('div', 'devs'); devs.setAttribute('role', 'tablist'); root.append(devs)
    const tabs = [], panes = [], box = make('div', 'panes')
    for (const [key, [label, build]] of Object.entries(PANES)) {
      const b = make('button', null, label); b.type = 'button'; b.setAttribute('role', 'tab'); b.dataset.k = key; devs.append(b); tabs.push(b)
      const p = make('div', 'pane'); p.dataset.k = key; p.append(...build()); box.append(p); panes.push(p)
    }
    root.append(box)
    // Room for the tallest one, so switching never changes the page's height (nor moves the scroll).
    const fit = () => {
      box.style.minHeight = ''
      const on = panes.find(p => p.classList.contains('on'))
      let tallest = 0
      for (const p of panes) { p.classList.add('on'); tallest = Math.max(tallest, p.offsetHeight); if (p !== on) p.classList.remove('on') }
      box.style.minHeight = tallest + 'px'
    }
    const show = k => { tabs.forEach(t => t.setAttribute('aria-selected', String(t.dataset.k === k))); panes.forEach(p => p.classList.toggle('on', p.dataset.k === k)) }
    tabs.forEach(t => t.addEventListener('click', () => show(t.dataset.k)))
    show(os); fit(); addEventListener('resize', fit); document.fonts?.ready.then(fit)
  }

  // Scroll → moments: `raw` follows the scroll, `s` holds on each moment then moves (recipes §1).
  function take(track, last, frame, hold = .34) {
    let s = null, rs = 0, prev = performance.now()
    const tick = now => {
      const dt = Math.min(.1, (now - prev) / 1000); prev = now
      const span = Math.max(1, track.offsetHeight - innerHeight)
      const raw = clamp01(scrollY / span) * last
      const k = Math.min(last - 1, Math.floor(raw))
      const target = raw >= last ? last : k + smooth(hold, 1, raw - k)
      const ease = Math.min(1, dt * (reduced ? 40 : 6))
      if (s === null) { s = target; rs = raw }
      s += (target - s) * ease; rs += (raw - rs) * ease
      frame(s, rs, now / 1000, dt)
      requestAnimationFrame(tick)
    }
    requestAnimationFrame(tick)
  }
  // A caption shown around moment(s) [from, to].
  function caption(el, s, from, to = from) {
    const d = s < from ? from - s : s > to ? s - to : 0
    const o = 1 - smooth(.06, .22, d)
    el.style.opacity = o; el.style.visibility = o < .02 ? 'hidden' : 'visible'
    return o
  }

  addEventListener('DOMContentLoaded', () => { const dl = document.querySelector('[data-download]'); if (dl) download(dl) })

  return { clamp01, smooth, mid, reduced, make, take, caption, os }
})()
