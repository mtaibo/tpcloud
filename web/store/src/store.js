import { computed, reactive, ref } from 'vue'

// Where the browser is and how it shows things, shared by the sidebar, the browser and the
// phone's tabs. Folders are addressed as (location, path) like the API.
function stored(key, fallback) {
  try { return localStorage.getItem(key) ?? fallback } catch { return fallback }
}
function persist(key, value) {
  try { localStorage.setItem(key, value) } catch { /* private mode */ }
}

export const store = reactive({
  user: null,
  location: 'external',
  path: '',
  history: [],
  index: -1,
  panel: 'files', // files | shares | browse (phone only)
  view: stored('tpstore-view', stored('gallery-mode', '0') === '1' ? 'icons' : 'list'),
  sortKey: stored('tpstore-sort', 'name'),
  sortAsc: stored('tpstore-sort-asc', '1') === '1',
  disk: null,
})

export const homePath = computed(() => (store.user ? `users/${store.user.email}` : ''))
export const canGoBack = computed(() => store.index > 0)
export const canGoForward = computed(() => store.index < store.history.length - 1)

export function navigate(location, path) {
  store.panel = 'files'
  const current = store.history[store.index]
  if (current && current.location === location && current.path === path) return
  const history = store.history.slice(0, store.index + 1)
  history.push({ location, path })
  if (history.length > 50) history.shift()
  store.history = history
  store.index = history.length - 1
  store.location = location
  store.path = path
}

function apply(index) {
  const s = store.history[index]
  store.index = index
  store.location = s.location
  store.path = s.path
  store.panel = 'files'
}
export function goBack() { if (canGoBack.value) apply(store.index - 1) }
export function goForward() { if (canGoForward.value) apply(store.index + 1) }

export function parentOf(path) {
  const parts = path.split('/').filter(Boolean)
  const root = rootOf(path)
  if (parts.join('/') === root) return null
  parts.pop()
  return parts.join('/')
}

export function rootOf(path) {
  if (path === 'shared' || path.startsWith('shared/')) return 'shared'
  return homePath.value
}

export function folderTitle(path) {
  if (!path) return 'Archivos'
  if (path === homePath.value) return 'Inicio'
  if (path === 'shared') return 'Compartido'
  return path.split('/').filter(Boolean).at(-1)
}

export function setView(view) {
  store.view = view
  persist('tpstore-view', view)
}

export function setSort(key) {
  if (store.sortKey === key) store.sortAsc = !store.sortAsc
  else { store.sortKey = key; store.sortAsc = key === 'name' || key === 'kind' }
  persist('tpstore-sort', store.sortKey)
  persist('tpstore-sort-asc', store.sortAsc ? '1' : '0')
}

export async function loadDisk() {
  try {
    const res = await fetch('/store/api/files/disk')
    if (res.ok) store.disk = await res.json()
  } catch { /* the meter just stays hidden */ }
}

// A search across the current folder, shared by the toolbar field and the list.
export const query = ref('')
