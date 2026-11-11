import { reactive } from 'vue'

// One alert at a time, asked for from anywhere and answered with a promise, like
// SwiftUI's .alert / .confirmationDialog. <AlertHost /> draws it.
export const alertState = reactive({ open: false, title: '', message: '', actions: [], resolve: null })

export function ask({ title, message = '', actions }) {
  return new Promise(resolve => {
    Object.assign(alertState, { open: true, title, message, actions, resolve })
  })
}

export function confirmDestructive(title, message, label = 'Eliminar') {
  return ask({
    title,
    message,
    actions: [
      { label: 'Cancelar', role: 'cancel', value: false },
      { label, role: 'destructive', value: true },
    ],
  })
}

export function answer(value) {
  const resolve = alertState.resolve
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
