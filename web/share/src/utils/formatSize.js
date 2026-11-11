export function formatSize(bytes) {
  if (bytes === null || bytes === undefined) return '—'
  if (bytes < 1000) return `${bytes} bytes`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes / 1000
  let unit = 0
  while (value >= 1000 && unit < units.length - 1) { value /= 1000; unit++ }
  return `${value.toLocaleString('es-ES', { maximumFractionDigits: value < 10 ? 1 : 0 })} ${units[unit]}`
}

const time = new Intl.DateTimeFormat('es-ES', { hour: '2-digit', minute: '2-digit' })
const day = new Intl.DateTimeFormat('es-ES', { day: 'numeric', month: 'short', year: 'numeric' })

/** Like Finder: "Hoy, 10:32", "Ayer, 18:05", "3 nov 2026, 9:12". */
export function formatDate(seconds) {
  if (!seconds) return '—'
  const d = new Date(seconds * 1000)
  const start = x => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime()
  const days = Math.round((start(new Date()) - start(d)) / 86400000)
  if (days === 0) return `Hoy, ${time.format(d)}`
  if (days === 1) return `Ayer, ${time.format(d)}`
  return `${day.format(d).replace('.', '')}, ${time.format(d)}`
}
