<script setup>
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Check } from 'lucide-vue-next'
import { closeMenu, menuState } from '../menu.js'

const el = ref(null)
const pos = ref({ left: '0px', top: '0px' })

watch(() => [menuState.open, menuState.x, menuState.y], async () => {
  if (!menuState.open) return
  pos.value = { left: '-9999px', top: '-9999px' }
  await nextTick()
  const rect = el.value.getBoundingClientRect()
  let x = menuState.anchor ? menuState.x - rect.width : menuState.x
  let y = menuState.y
  x = Math.max(8, Math.min(x, window.innerWidth - rect.width - 8))
  if (y + rect.height > window.innerHeight - 8) y = Math.max(8, (menuState.anchor ? menuState.anchor.top - 6 : y) - rect.height)
  pos.value = { left: `${x}px`, top: `${y}px` }
})

function run(item) {
  if (item.disabled) return
  closeMenu()
  item.action?.()
}

function onDown(e) {
  if (menuState.open && el.value && !el.value.contains(e.target)) closeMenu()
}
function onKey(e) { if (e.key === 'Escape') closeMenu() }

onMounted(() => {
  document.addEventListener('pointerdown', onDown, true)
  document.addEventListener('keydown', onKey)
  window.addEventListener('resize', closeMenu)
  window.addEventListener('blur', closeMenu)
})
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onDown, true)
  document.removeEventListener('keydown', onKey)
  window.removeEventListener('resize', closeMenu)
  window.removeEventListener('blur', closeMenu)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="menu">
      <div v-if="menuState.open" ref="el" class="menu" :style="pos" role="menu" @contextmenu.prevent>
        <p v-if="menuState.title" class="menu-title">{{ menuState.title }}</p>
        <template v-for="(item, i) in menuState.items" :key="i">
          <div v-if="item.separator" class="menu-sep" />
          <button
            v-else
            type="button"
            role="menuitem"
            class="menu-item"
            :class="{ danger: item.danger }"
            :disabled="item.disabled"
            @click="run(item)"
          >
            <span class="check"><Check v-if="item.checked" :size="14" :stroke-width="2.6" /></span>
            <span class="menu-label">{{ item.label }}</span>
            <component :is="item.icon" v-if="item.icon" class="menu-icon" :size="17" :stroke-width="1.8" />
          </button>
        </template>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.menu {
  position: fixed;
  z-index: 300;
  min-width: 230px;
  max-width: 300px;
  padding: 6px;
  border-radius: 14px;
  background: var(--glass-strong);
  backdrop-filter: blur(30px) saturate(190%);
  -webkit-backdrop-filter: blur(30px) saturate(190%);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.2), 0 0 0 0.5px rgba(0, 0, 0, 0.12), inset 0 0.5px 0 var(--glass-edge);
  transform-origin: top left;
}
@media (prefers-color-scheme: dark) {
  .menu { box-shadow: 0 12px 40px rgba(0, 0, 0, 0.55), 0 0 0 0.5px rgba(255, 255, 255, 0.14); }
}
.menu-title {
  padding: 6px 10px 6px 30px;
  font-size: 12px;
  font-weight: 600;
  color: var(--label-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.menu-item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 30px;
  padding: 4px 10px 4px 6px;
  border-radius: 8px;
  font-size: 14px;
  letter-spacing: -0.15px;
  text-align: left;
  color: var(--label);
}
.menu-item:hover:not(:disabled), .menu-item:focus-visible { background: var(--accent); color: #fff; outline: none; }
.menu-item:hover:not(:disabled) .menu-icon { color: #fff; }
.menu-item:disabled { opacity: 0.35; cursor: default; }
.menu-item.danger { color: var(--red); }
.menu-item.danger:hover { background: var(--red); color: #fff; }
.check { width: 18px; flex-shrink: 0; display: grid; place-items: center; }
.menu-label { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.menu-icon { flex-shrink: 0; color: var(--label-2); }
.menu-item.danger .menu-icon { color: inherit; }
.menu-sep { height: 0.5px; margin: 5px 10px; background: var(--separator); }

@media (pointer: coarse) {
  .menu { min-width: 250px; padding: 0; border-radius: 16px; overflow: hidden; }
  .menu-item { min-height: 46px; padding: 0 16px 0 10px; border-radius: 0; font-size: 17px; }
  .menu-item + .menu-item { box-shadow: inset 0 0.5px 0 var(--separator); }
  .menu-sep { height: 8px; margin: 0; background: color-mix(in srgb, var(--label) 8%, transparent); }
  .menu-icon { width: 20px; height: 20px; color: var(--label); }
  .menu-item.danger .menu-icon { color: var(--red); }
  .menu-item:hover:not(:disabled) { background: transparent; color: inherit; }
  .menu-item:hover:not(:disabled) .menu-icon { color: var(--label); }
  .menu-item:active:not(:disabled) { background: var(--fill-3); }
  .menu-item.danger:hover { background: transparent; color: var(--red); }
  .menu-title { padding: 12px 16px 8px; font-size: 13px; font-weight: 400; text-align: center; }
}

.menu-enter-active { transition: opacity 0.12s, transform 0.18s var(--spring); }
.menu-leave-active { transition: opacity 0.12s; }
.menu-enter-from { opacity: 0; transform: scale(0.94); }
.menu-leave-to { opacity: 0; }
</style>
