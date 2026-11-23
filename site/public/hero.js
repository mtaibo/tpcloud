// C · The file: tpcloud, ~75 MB, opens into what it really carries (0.4.0, arm64), and the six processes come on.
(() => {
  const v = document.getElementById('visual'); if (!v) return
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches
  const make = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text != null) e.textContent = text; return e }
  const wrap = make('div', 'vfile'), doc = make('div', 'doc'), stream = make('div', 'stream')
  doc.append(make('b', null, 'tpcloud'), make('span', null, '~75 MB'))
  wrap.append(doc, stream); v.append(wrap)
  const PROCS = ['gateway', 'auth', 'store-api', 'home-api', 'mqtt-broker', 'cloudflared']
  const grid = make('div', 'procs'); v.append(grid)
  const procs = PROCS.map(p => { const e = make('div', 'proc'); e.append(make('i', 'vdot'), make('span', null, p)); grid.append(e); return e })
  // What's really inside, with the sizes of the arm64 build.
  const FILES = [['python/', 'CPython 3.12.16'], ['py/auth/', 'FastAPI, SQLModel, webauthn'], ['py/store/', 'Pillow, pillow-heif'], ['py/home/', 'paho-mqtt'], ['code/', 'auth, store, home'],
    ['web/', 'eight web apps · 13 MB'], ['bin/caddy', '46 MB'], ['bin/mosquitto', '456 KB'], ['bin/ffmpeg', '10 MB'], ['bin/ffprobe', '10 MB'], ['bin/rsync', '980 KB'], ['bin/cloudflared', '37 MB'],
    ['bin/btrfs', '1.4 MB'], ['bin/cryptsetup', '4.9 MB'], ['bin/smartctl', '2 MB'], ['', '7,341 files']]
  const sleep = ms => new Promise(r => setTimeout(r, reduced ? 0 : ms))
  ;(async () => {
    for (;;) {
      stream.replaceChildren(); procs.forEach(p => p.classList.remove('on')); doc.classList.remove('open')
      await sleep(1200)
      doc.classList.add('open')
      for (const [path, what] of FILES) {
        const line = make('div'); if (path) line.append(make('b', null, path.padEnd(16, ' '))); line.append(document.createTextNode(what))
        stream.append(line); while (stream.children.length > 7) stream.firstChild.remove()
        await sleep(330)
      }
      await sleep(400)
      for (const p of procs) { p.classList.add('on'); await sleep(260) }
      await sleep(4200)
    }
  })()
})()
