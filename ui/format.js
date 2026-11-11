// Dates, devices and sign-in methods in plain Spanish.

const relative = new Intl.RelativeTimeFormat('es', { numeric: 'auto' })
const dateTime = new Intl.DateTimeFormat('es-ES', { day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' })
const dateOnly = new Intl.DateTimeFormat('es-ES', { day: 'numeric', month: 'long', year: 'numeric' })
const dayMonth = new Intl.DateTimeFormat('es-ES', { weekday: 'long', day: 'numeric', month: 'long' })
const timeOnly = new Intl.DateTimeFormat('es-ES', { hour: '2-digit', minute: '2-digit' })

function toDate(value) { return value instanceof Date ? value : new Date(value) }

export function ago(value) {
  if (!value) return ''
  const seconds = (toDate(value).getTime() - Date.now()) / 1000
  const abs = Math.abs(seconds)
  if (abs < 60) return seconds <= 0 ? 'ahora mismo' : 'en unos segundos'
  if (abs < 3600) return relative.format(Math.round(seconds / 60), 'minute')
  if (abs < 86400) return relative.format(Math.round(seconds / 3600), 'hour')
  if (abs < 86400 * 30) return relative.format(Math.round(seconds / 86400), 'day')
  if (abs < 86400 * 365) return relative.format(Math.round(seconds / (86400 * 30)), 'month')
  return relative.format(Math.round(seconds / (86400 * 365)), 'year')
}

export const formatDateTime = value => (value ? dateTime.format(toDate(value)) : '')
export const formatDate = value => (value ? dateOnly.format(toDate(value)) : '')
export const formatTime = value => (value ? timeOnly.format(toDate(value)) : '')

export function dayHeading(value) {
  const d = toDate(value)
  const today = new Date()
  const start = x => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime()
  const days = Math.round((start(today) - start(d)) / 86400000)
  if (days === 0) return 'Hoy'
  if (days === 1) return 'Ayer'
  const text = dayMonth.format(d)
  const withYear = d.getFullYear() !== today.getFullYear() ? `${text} de ${d.getFullYear()}` : text
  return withYear.charAt(0).toUpperCase() + withYear.slice(1)
}

export function deviceIcon(kind) {
  return { phone: 'iphone', tablet: 'ipad', desktop: 'laptop' }[kind] || 'globe'
}

export const methodNames = {
  passkey: 'Passkey',
  password: 'Contraseña',
  qr: 'Código QR',
}

export function methodSentence(session) {
  if (session.auth_method === 'qr') {
    return session.approved_by ? `Con un código QR aprobado desde ${session.approved_by}` : 'Con un código QR'
  }
  if (session.auth_method === 'passkey') return 'Con passkey'
  if (session.auth_method === 'password') return 'Con contraseña'
  return session.auth_method
}

export function where(item) {
  const location = item.location && item.location !== item.ip_address ? item.location : ''
  return location || item.last_ip || item.ip_address || 'Ubicación desconocida'
}

export function initials(name = '') {
  return name.split(/\s+/).filter(Boolean).slice(0, 2).map(w => w[0].toUpperCase()).join('') || '?'
}
