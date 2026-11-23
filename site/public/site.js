// What to download from here: this device first, the right file for it, and the copy button.
(async () => {
  const ua = navigator.userAgent
  const os = /iPhone|iPad/.test(ua) || (/Mac/.test(ua) && navigator.maxTouchPoints > 1) ? 'ios'
    : /Android/.test(ua) ? 'android' : /Mac/.test(ua) ? 'mac' : /Windows/.test(ua) ? 'windows' : /Linux|X11|CrOS/.test(ua) ? 'linux' : 'other'
  let arch = /aarch64|arm64|armv8/i.test(ua) ? 'arm64' : /x86_64|amd64|Win64|x64/i.test(ua) ? 'amd64' : null
  try {
    const v = await navigator.userAgentData?.getHighEntropyValues?.(['architecture'])
    if (v?.architecture === 'arm') arch = 'arm64'; else if (v?.architecture === 'x86') arch = 'amd64'
  } catch {}

  // The desktop app for Linux: this machine's architecture, a PC's when it can't be told.
  const appArch = arch || 'amd64'
  const SIZES = { arm64: '20 MB', amd64: '22 MB' }
  for (const a of document.querySelectorAll('[data-app="linux"]')) {
    a.href = `/download/app/linux/${appArch}`
    a.querySelector('small').textContent = `${appArch} · ${SIZES[appArch]}`
  }

  // This device's row goes first and says so.
  const list = document.querySelector('[data-devices]')
  const row = list && [...list.children].find(li => li.dataset.os.split(' ').includes(os))
  if (row) { row.classList.add('here'); list.prepend(row) }

  // The hero's second button goes straight to what this device needs.
  const apps = document.querySelector('[data-hero="apps"]')
  if (apps && os === 'linux') { apps.href = `/download/app/linux/${appArch}`; apps.textContent = 'Download the app for Linux' }

  const note = document.querySelector('[data-note]')
  if (note) note.textContent = {
    ios: 'Your cloud lives on a Raspberry Pi or a Linux PC at home; on this iPhone you’ll use the TPCloud app.',
    android: 'Your cloud lives on a Raspberry Pi or a Linux PC at home; on this phone, its address in the browser.',
    mac: 'Your cloud lives on a Raspberry Pi or a Linux PC at home; on this Mac, the TPCloud app.',
    windows: 'Your cloud lives on a Raspberry Pi or a Linux PC at home; on this PC, its address in the browser.',
    linux: 'On the machine that will be your cloud, the command below. On this computer, the app.',
    other: 'Your cloud lives on a Raspberry Pi or a Linux PC at home.',
  }[os]

  for (const btn of document.querySelectorAll('.copy')) btn.addEventListener('click', async () => {
    const text = btn.closest('.cmd').querySelector('code').dataset.copy
    try { await navigator.clipboard.writeText(text) } catch {
      const r = document.createRange(); r.selectNodeContents(btn.closest('.cmd').querySelector('code')); getSelection().removeAllRanges(); getSelection().addRange(r)
    }
    btn.textContent = 'Copied'; btn.classList.add('done')
    setTimeout(() => { btn.textContent = 'Copy'; btn.classList.remove('done') }, 1600)
  })
})()
