// Dates, devices and sign-in methods in plain words, in the language chosen in Cuenta.
import { lang, locale, t } from './i18n.js'

// Formatters for the current language, made again only when it changes.
let cachedFor = null
let f = null
function fmt() {
  if (cachedFor !== lang.value) {
    const l = locale()
    f = {
      relative: new Intl.RelativeTimeFormat(lang.value, { numeric: 'auto' }),
      dateTime: new Intl.DateTimeFormat(l, { day: 'numeric', month: 'long', year: 'numeric', hour: '2-digit', minute: '2-digit' }),
      dateOnly: new Intl.DateTimeFormat(l, { day: 'numeric', month: 'long', year: 'numeric' }),
      dayMonth: new Intl.DateTimeFormat(l, { weekday: 'long', day: 'numeric', month: 'long' }),
      timeOnly: new Intl.DateTimeFormat(l, { hour: '2-digit', minute: '2-digit' }),
    }
    cachedFor = lang.value
  }
  return f
}

function toDate(value) { return value instanceof Date ? value : new Date(value) }

export function ago(value) {
  if (!value) return ''
  const { relative } = fmt()
  const seconds = (toDate(value).getTime() - Date.now()) / 1000
  const abs = Math.abs(seconds)
  if (abs < 60) return seconds <= 0 ? t('just now', 'ahora mismo') : t('in a few seconds', 'en unos segundos')
  if (abs < 3600) return relative.format(Math.round(seconds / 60), 'minute')
  if (abs < 86400) return relative.format(Math.round(seconds / 3600), 'hour')
  if (abs < 86400 * 30) return relative.format(Math.round(seconds / 86400), 'day')
  if (abs < 86400 * 365) return relative.format(Math.round(seconds / (86400 * 30)), 'month')
  return relative.format(Math.round(seconds / (86400 * 365)), 'year')
}

export const formatDateTime = value => (value ? fmt().dateTime.format(toDate(value)) : '')
export const formatDate = value => (value ? fmt().dateOnly.format(toDate(value)) : '')
export const formatTime = value => (value ? fmt().timeOnly.format(toDate(value)) : '')

export function dayHeading(value) {
  const d = toDate(value)
  const today = new Date()
  const start = x => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime()
  const days = Math.round((start(today) - start(d)) / 86400000)
  if (days === 0) return t('Today', 'Hoy')
  if (days === 1) return t('Yesterday', 'Ayer')
  const text = fmt().dayMonth.format(d)
  const withYear = d.getFullYear() !== today.getFullYear() ? `${text}${t(' ', ' de ')}${d.getFullYear()}` : text
  return withYear.charAt(0).toUpperCase() + withYear.slice(1)
}

export function deviceIcon(kind) {
  return { phone: 'iphone', tablet: 'ipad', desktop: 'laptop' }[kind] || 'globe'
}

export const methodNames = {
  passkey: 'Passkey',
  get password() { return t('Password', 'Contraseña') },
  get qr() { return t('QR code', 'Código QR') },
}

export function methodSentence(session) {
  if (session.auth_method === 'qr') {
    return session.approved_by
      ? t(`With a QR code approved from ${session.approved_by}`, `Con un código QR aprobado desde ${session.approved_by}`)
      : t('With a QR code', 'Con un código QR')
  }
  if (session.auth_method === 'passkey') return t('With a passkey', 'Con passkey')
  if (session.auth_method === 'password') return t('With a password', 'Con contraseña')
  return session.auth_method
}

export function where(item) {
  const location = item.location && item.location !== item.ip_address ? item.location : ''
  return location || item.last_ip || item.ip_address || t('Unknown location', 'Ubicación desconocida')
}

export function initials(name = '') {
  return name.split(/\s+/).filter(Boolean).slice(0, 2).map(w => w[0].toUpperCase()).join('') || '?'
}
