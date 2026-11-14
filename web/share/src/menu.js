import { reactive } from 'vue'

// One context menu at a time, opened at a point (right click, long press) or under a button.
// Items: { label, icon, action, danger, disabled, checked } or { separator: true }.
export const menuState = reactive({ open: false, x: 0, y: 0, anchor: null, items: [], title: '' })

export function openMenu(at, items, title = '') {
  const anchor = at instanceof Element ? at.getBoundingClientRect() : null
  Object.assign(menuState, {
    open: true,
    items: items.filter(Boolean),
    title,
    anchor,
    x: anchor ? anchor.right : at.clientX,
    y: anchor ? anchor.bottom + 6 : at.clientY,
  })
}

export function closeMenu() {
  menuState.open = false
}
