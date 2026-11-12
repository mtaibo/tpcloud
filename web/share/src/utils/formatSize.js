export function formatSize(bytes) {
  if (bytes === null || bytes === undefined) return '—'
  if (bytes < 1000) return `${bytes} bytes`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes / 1000
  let unit = 0
  while (value >= 1000 && unit < units.length - 1) { value /= 1000; unit++ }
  return `${value.toLocaleString('en-US', { maximumFractionDigits: value < 10 ? 1 : 0 })} ${units[unit]}`
}

const time = new Intl.DateTimeFormat('en-US', { hour: 'numeric', minute: '2-digit' })
const day = new Intl.DateTimeFormat('en-US', { day: 'numeric', month: 'short', year: 'numeric' })

/** Like Finder: "Today, 10:32 AM", "Yesterday, 6:05 PM", "Nov 3, 2026, 9:12 AM". */
export function formatDate(seconds) {
  if (!seconds) return '—'
  const d = new Date(seconds * 1000)
  const start = x => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime()
  const days = Math.round((start(new Date()) - start(d)) / 86400000)
  if (days === 0) return `Today, ${time.format(d)}`
  if (days === 1) return `Yesterday, ${time.format(d)}`
  return `${day.format(d)}, ${time.format(d)}`
}
