// C · The file: tpcloud, 66 MB, opens into what it really carries, and the six processes come on.
(() => {
  const v = document.getElementById('visual'); if (!v) return
  const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches
  const make = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text != null) e.textContent = text; return e }
  const wrap = make('div', 'vfile'), doc = make('div', 'doc'), stream = make('div', 'stream')
  doc.append(make('b', null, 'tpcloud'), make('span', null, '66 MB'))
  wrap.append(doc, stream); v.append(wrap)
  const PROCS = ['gateway', 'auth', 'store-api', 'home-api', 'mqtt-broker', 'cloudflared']
  const grid = make('div', 'procs'); v.append(grid)
  const procs = PROCS.map(p => { const e = make('div', 'proc'); e.append(make('i', 'vdot'), make('span', null, p)); grid.append(e); return e })
  // What's really inside, with the sizes of the arm64 build.
  const FILES = [['python/', 'CPython 3.12.16'], ['py/auth/', 'FastAPI, SQLModel, webauthn'], ['py/store/', 'Pillow, pillow-heif'], ['py/home/', 'paho-mqtt'], ['code/', 'auth, store, home'],
    ['web/', 'seven web apps · 1.6 MB'], ['bin/caddy', '46 MB'], ['bin/mosquitto', '456 KB'], ['bin/ffmpeg', '11 MB'], ['bin/ffprobe', '11 MB'], ['bin/rsync', '980 KB'], ['bin/cloudflared', '36 MB'], ['', '6,922 files']]
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
