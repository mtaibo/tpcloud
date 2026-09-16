import { ref } from 'vue'

const STORAGE_KEY = 'tpcloud-favourites'

export function useFavourites() {
  const favourites = ref([])

  function load() {
    try {
      const stored = localStorage.getItem(STORAGE_KEY)
      if (stored) favourites.value = JSON.parse(stored)
    } catch {}
  }

  function save() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(favourites.value))
  }

  function derivLabel(loc, path, userEmail) {
    if (!path) return loc === 'server' ? 'Server Root' : 'Disk Root'
    const parts = path.split('/').filter(Boolean)
    const last = parts[parts.length - 1]
    if (userEmail && path === `users/${userEmail}`) return 'Personal'
    if (last === 'shared') return 'Shared'
    return last.charAt(0).toUpperCase() + last.slice(1)
  }

  function add(loc, path, userEmail) {
    const id = `${loc}:${path}`
    if (favourites.value.some(f => f.id === id)) return false
    favourites.value.push({ id, label: derivLabel(loc, path, userEmail), location: loc, path })
    save()
    return true
  }

  function remove(id) {
    favourites.value = favourites.value.filter(f => f.id !== id)
    save()
  }

  return { favourites, load, add, remove, derivLabel }
}
