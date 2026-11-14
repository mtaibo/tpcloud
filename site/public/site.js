// The download for this machine, and the copy button.
(async () => {
  const BASE = 'https://tpcloud.migueltaibo.com/download/linux/'
  const ua = navigator.userAgent
  const os = /iPhone|iPad/.test(ua) ? 'ios' : /Mac/.test(ua) ? 'mac' : /Windows/.test(ua) ? 'windows' : /Android/.test(ua) ? 'android' : /Linux/.test(ua) ? 'linux' : 'other'
  let arch = /aarch64|arm64|armv8/i.test(ua) ? 'arm64' : /x86_64|amd64/i.test(ua) ? 'amd64' : null
  try {
    const v = await navigator.userAgentData?.getHighEntropyValues?.(['architecture'])
    if (v?.architecture === 'arm') arch = 'arm64'; else if (v?.architecture === 'x86') arch = 'amd64'
  } catch {}
  const NAMES = { arm64: 'Raspberry Pi and ARM', amd64: 'PCs (Intel and AMD)' }
  // On a Linux machine we know which file; anywhere else the Pi's is the likely one.
  const main = os === 'linux' && arch ? arch : 'arm64'
  const other = main === 'arm64' ? 'amd64' : 'arm64'
  for (const el of document.querySelectorAll('[data-dl="main"]')) {
    el.href = BASE + main
    el.querySelector('.label').textContent = os === 'linux' && arch ? 'Download for this machine' : 'Download for Raspberry Pi'
    el.querySelector('small').textContent = `Linux ${main} · 66 MB`
  }
  for (const el of document.querySelectorAll('[data-dl="other"]')) { el.href = BASE + other; el.textContent = `${NAMES[other]} (${other})` }
  for (const el of document.querySelectorAll('[data-dl]:not([data-dl="main"]):not([data-dl="other"])')) el.href = BASE + el.dataset.dl
  const note = document.querySelector('[data-note]')
  if (note) note.textContent = {
    ios: 'This goes on the machine at home that will be your cloud. On your iPhone you’ll use the TPCloud app, coming to the App Store.',
    mac: 'This goes on the machine at home that will be your cloud: a Raspberry Pi or an old PC with Linux. The Mac app is coming to the App Store.',
    windows: 'This goes on a Raspberry Pi or an old PC with Linux. A Windows server comes later.',
    android: 'This goes on the machine at home that will be your cloud: a Raspberry Pi or an old PC with Linux.',
    linux: 'Run the command on the machine itself. It asks a few questions and starts everything.',
    other: 'This goes on the machine at home that will be your cloud: a Raspberry Pi or an old PC with Linux.',
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
