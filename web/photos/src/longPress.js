/*
 * v-long-press="point => …": holding a finger on an element calls the handler with where the
 * finger is, for opening the context menu on phones. iOS Safari never sends `contextmenu` for a
 * long press, and its own callout on a thumbnail would save the small preview, not the original.
 *
 * The tap that ends a long press is swallowed, so it doesn't also open what was pressed. While
 * the finger is down the element gets `long-pressing`, for a slight press-in effect.
 */
const HOLD_MS = 500
const MOVE_TOLERANCE = 10

export const vLongPress = {
  mounted(el, binding) {
    let timer = null
    let pressTimer = null
    let start = null
    let fired = false

    function clear() {
      clearTimeout(timer)
      clearTimeout(pressTimer)
      timer = pressTimer = null
      el.classList.remove('long-pressing')
    }

    function onTouchStart(e) {
      if (e.touches.length !== 1) return clear()
      fired = false
      start = { x: e.touches[0].clientX, y: e.touches[0].clientY }
      pressTimer = setTimeout(() => el.classList.add('long-pressing'), 120)
      timer = setTimeout(() => {
        clear()
        fired = true
        el._longPressHandler?.(start)
      }, HOLD_MS)
    }

    function onTouchMove(e) {
      if (!timer || !start) return
      const t = e.touches[0]
      if (Math.hypot(t.clientX - start.x, t.clientY - start.y) > MOVE_TOLERANCE) clear()
    }

    function onTouchEnd(e) {
      clear()
      // Cancels the click the browser would synthesize from this touch.
      if (fired) e.preventDefault()
    }

    // Android does send `contextmenu` on a long press; the menu is already open by then.
    function onContextMenu(e) {
      if (fired) {
        e.preventDefault()
        e.stopPropagation()
      }
    }

    function onClick(e) {
      if (!fired) return
      fired = false
      e.preventDefault()
      e.stopImmediatePropagation()
    }

    el._longPressHandler = binding.value
    el._longPressCleanup = () => {
      clear()
      el.removeEventListener('touchstart', onTouchStart)
      el.removeEventListener('touchmove', onTouchMove)
      el.removeEventListener('touchend', onTouchEnd)
      el.removeEventListener('touchcancel', clear)
      el.removeEventListener('contextmenu', onContextMenu, true)
      el.removeEventListener('click', onClick, true)
    }
    el.addEventListener('touchstart', onTouchStart, { passive: true })
    el.addEventListener('touchmove', onTouchMove, { passive: true })
    el.addEventListener('touchend', onTouchEnd)
    el.addEventListener('touchcancel', clear)
    el.addEventListener('contextmenu', onContextMenu, true)
    el.addEventListener('click', onClick, true)
  },
  updated(el, binding) {
    el._longPressHandler = binding.value
  },
  unmounted(el) {
    el._longPressCleanup?.()
  },
}
