// The download section: this device's tab first, the right files for it, and the copy buttons.
(async () => {
  document.documentElement.classList.add('js')
  const ua = navigator.userAgent
  const os = /iPhone|iPad/.test(ua) || (/Mac/.test(ua) && navigator.maxTouchPoints > 1) ? 'iphone'
    : /Android/.test(ua) ? 'other' : /Mac/.test(ua) ? 'mac' : /Windows/.test(ua) ? 'other' : /Linux|X11|CrOS/.test(ua) ? 'linux' : 'mac'
  let arch = /aarch64|arm64|armv8/i.test(ua) ? 'arm64' : /x86_64|amd64|Win64|x64/i.test(ua) ? 'amd64' : null
  try {
    const v = await navigator.userAgentData?.getHighEntropyValues?.(['architecture'])
    if (v?.architecture === 'arm') arch = 'arm64'; else if (v?.architecture === 'x86') arch = 'amd64'
  } catch {}

  // Tabs.
  const tabs = [...document.querySelectorAll('[data-tab]')]
  const panels = [...document.querySelectorAll('[data-panel]')]
  const show = name => {
    for (const t of tabs) t.setAttribute('aria-selected', String(t.dataset.tab === name))
    for (const p of panels) p.classList.toggle('on', p.dataset.panel === name)
  }
  for (const t of tabs) t.addEventListener('click', () => show(t.dataset.tab))
  show(os)

  // Linux files for this machine (a PC's when it can't be told).
  const a = arch || 'amd64'
  const SIZES = { app: { arm64: '20 MB', amd64: '22 MB' }, cli: { arm64: '2 MB', amd64: '2 MB' } }
  for (const el of document.querySelectorAll('[data-file]')) {
    const kind = el.dataset.file
    el.href = `/download/${kind}/linux/${a}`
    el.querySelector('small').textContent = `${a} · ${SIZES[kind][a]}`
  }

  const note = document.querySelector('[data-note]')
  if (note) note.textContent = {
    mac: 'On this Mac: the app on TestFlight, and the tpcloud command with one line.',
    linux: 'On this computer: the TPCloud app and the tpcloud command, with one line.',
    iphone: 'On this iPhone: the TPCloud app, on TestFlight.',
    other: 'On this device: your cloud’s address in the browser.',
  }[os]

  for (const btn of document.querySelectorAll('.copy')) btn.addEventListener('click', async () => {
    const code = btn.closest('.cmd').querySelector('code')
    try { await navigator.clipboard.writeText(code.dataset.copy) } catch {
      const r = document.createRange(); r.selectNodeContents(code); getSelection().removeAllRanges(); getSelection().addRange(r)
    }
    btn.textContent = 'Copied'; btn.classList.add('done')
    setTimeout(() => { btn.textContent = 'Copy'; btn.classList.remove('done') }, 1600)
  })
})()
