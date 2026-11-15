import { ref } from 'vue'

// Favourite folders live in store-api (/api/favorites), one list per account, so every browser,
// session and the app show the same ones. Browsers used to keep their own list in localStorage:
// the first load hands it to the server and forgets it.
const LEGACY_KEY = 'tpcloud-favourites'
const API = '/store/api/favorites'
const favourites = ref([])
let loading = null
let watching = false

async function request(url, options) {
  const res = await fetch(url, options)
  if (!res.ok) throw Object.assign(new Error(`HTTP ${res.status}`), { status: res.status })
  return res.json()
}

async function migrateLegacy() {
  let stored
  try { stored = JSON.parse(localStorage.getItem(LEGACY_KEY) || 'null') } catch { stored = null }
  if (!Array.isArray(stored) || !stored.length) return null
  const items = stored.filter(f => f && f.path).map(f => ({ location: f.location || 'external', path: f.path }))
  try {
    const data = await request(API, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ items }) })
    forgetLegacy()
    return data.favorites
  } catch (e) {
    // Refused (a folder it can no longer reach): drop the old list rather than retry forever.
    // Offline or a server error: keep it for the next load.
    if (e.status >= 400 && e.status < 500 && e.status !== 401) forgetLegacy()
    return null
  }
}

function forgetLegacy() {
  try { localStorage.removeItem(LEGACY_KEY) } catch {}
}

function fetchAll() {
  loading ??= (async () => {
    try {
      favourites.value = (await migrateLegacy()) ?? (await request(API))
    } catch { /* keeps what it had */ } finally {
      loading = null
    }
  })()
  return loading
}

export function useFavourites() {
  function load() {
    // Coming back to the tab picks up what another device or session changed.
    if (!watching) {
      watching = true
      document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') fetchAll() })
    }
    return fetchAll()
  }

  async function add(loc, path) {
    const id = `${loc}:${path}`
    if (favourites.value.some(f => f.id === id)) return false
    const data = await request(API, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ items: [{ location: loc, path }] }) })
    favourites.value = data.favorites
    return data.added.includes(id)
  }

  async function remove(id) {
    const fav = favourites.value.find(f => f.id === id)
    if (!fav) return
    favourites.value = favourites.value.filter(f => f.id !== id)
    try {
      const data = await request(`${API}?${new URLSearchParams({ path: fav.path, location: fav.location })}`, { method: 'DELETE' })
      favourites.value = data.favorites
    } catch {
      fetchAll()
    }
  }

  // The icon itself is saved by /files/icon; this only keeps the list in step without a reload.
  function updateIconName(id, iconName) {
    const fav = favourites.value.find(f => f.id === id)
    if (fav) fav.icon_name = iconName || null
  }

  return { favourites, load, add, remove, updateIconName }
}
