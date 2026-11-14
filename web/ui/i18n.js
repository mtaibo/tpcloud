import { ref } from 'vue'

// English or Spanish, chosen in Cuenta → Idioma and kept on the account (auth's /me reports it).
// Every app on this host shares localStorage, so the last choice is known before /me answers and
// pages open in the right language straight away. Strings live where they're used, as pairs:
// t('Files', 'Archivos'). Reading `lang` inside t() is what makes templates follow a change.

const KEY = 'tpcloud-lang'
const LANGS = ['en', 'es']

function initial() {
  try {
    const saved = localStorage.getItem(KEY)
    if (LANGS.includes(saved)) return saved
  } catch { /* storage blocked: fall back to the browser */ }
  return navigator.language?.toLowerCase().startsWith('es') ? 'es' : 'en'
}

export const lang = ref(initial())
let locked = false
document.documentElement.lang = lang.value

function apply(value) {
  lang.value = value
  document.documentElement.lang = value
}

export function setLang(value) {
  if (locked || !LANGS.includes(value)) return
  apply(value)
  try { localStorage.setItem(KEY, value) } catch { /* only this page keeps it */ }
}

// The account's choice from /me, which wins over whatever this browser remembered.
export function syncLang(me) {
  if (me?.language) setLang(me.language)
}

// Pages that are always in one language (the sign-in and share pages are English only).
export function lockLang(value) {
  apply(value)
  locked = true
}

export const t = (en, es) => (lang.value === 'es' ? es : en)

// For Intl formatters: day-month order in English too, as written in Spain.
export const locale = () => (lang.value === 'es' ? 'es-ES' : 'en-GB')
