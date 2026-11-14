import { reactive } from 'vue'
import { t } from './i18n.js'

// One alert at a time, asked for from anywhere and answered with a promise, like
// SwiftUI's .alert / .confirmationDialog. <AlertHost /> draws it.
export const alertState = reactive({ open: false, title: '', message: '', actions: [], field: null, resolve: null })

export function ask({ title, message = '', actions, field = null }) {
  return new Promise(resolve => {
    Object.assign(alertState, { open: true, title, message, actions, field, resolve })
  })
}

// An alert with a text field (new folder, rename…). Resolves to the text, or null if cancelled.
// `select` is how much of the text starts selected (e.g. the name without its extension).
export function prompt({ title, message = '', value = '', placeholder = '', confirm = t('OK', 'Aceptar'), select = null }) {
  return ask({
    title,
    message,
    field: { value, placeholder, select },
    actions: [
      { label: t('Cancel', 'Cancelar'), role: 'cancel', value: null },
      { label: confirm, role: 'default', value: '__field__' },
    ],
  })
}

export function confirmDestructive(title, message, label = t('Delete', 'Eliminar')) {
  return ask({
    title,
    message,
    actions: [
      { label: t('Cancel', 'Cancelar'), role: 'cancel', value: false },
      { label, role: 'destructive', value: true },
    ],
  })
}

export function answer(value) {
  const resolve = alertState.resolve
  if (value === '__field__') {
    const text = alertState.field.value.trim()
    if (!text) return
    value = text
  }
  alertState.open = false
  alertState.resolve = null
  resolve?.(value)
}

// A short message floating over the bottom of the page.
export const toastState = reactive({ text: '', icon: '', visible: false, timer: null })

export function toast(text, icon = 'check-circle') {
  clearTimeout(toastState.timer)
  Object.assign(toastState, { text, icon, visible: true })
  toastState.timer = setTimeout(() => { toastState.visible = false }, 2600)
}
