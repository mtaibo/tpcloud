<script setup>
import { nextTick, ref, watch } from 'vue'
import { alertState, answer, toastState } from './dialog.js'
import Icon from './Icon.vue'

const input = ref(null)

watch(() => alertState.open, async open => {
  if (!open || !alertState.field) return
  await nextTick()
  const el = input.value
  if (!el) return
  el.focus()
  const end = alertState.field.select ?? alertState.field.value.length
  el.setSelectionRange(0, end)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="alert">
      <div v-if="alertState.open" class="overlay" style="align-items: center" @click.self="answer(alertState.field ? null : false)">
        <div class="alert" role="alertdialog" aria-modal="true">
          <h2>{{ alertState.title }}</h2>
          <p v-if="alertState.message" class="secondary">{{ alertState.message }}</p>
          <input
            v-if="alertState.field"
            ref="input"
            v-model="alertState.field.value"
            class="alert-field"
            :placeholder="alertState.field.placeholder"
            spellcheck="false"
            autocomplete="off"
            @keydown.enter="answer('__field__')"
            @keydown.esc="answer(null)"
          />
          <div class="alert-actions" :class="{ pair: alertState.actions.length === 2 }">
            <button
              v-for="a in alertState.actions"
              :key="a.label"
              type="button"
              class="btn"
              :class="a.role === 'cancel' ? 'btn-cancel' : a.role === 'destructive' ? 'btn-prominent destructive' : 'btn-prominent'"
              :disabled="a.value === '__field__' && !alertState.field.value.trim()"
              @click="answer(a.value)"
            >{{ a.label }}</button>
          </div>
        </div>
      </div>
    </Transition>
    <Transition name="toast">
      <div v-if="toastState.visible" class="toast glass" role="status">
        <Icon :name="toastState.icon" :size="20" :style="{ color: toastState.icon === 'warning' ? 'var(--orange)' : 'var(--green)' }" />
        {{ toastState.text }}
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.alert-field {
  width: 100%;
  margin-top: 14px;
  padding: 10px 14px;
  border: none;
  border-radius: 12px;
  outline: none;
  font-size: 15px;
  color: var(--label);
  background: var(--fill-3);
}
.alert-field:focus { box-shadow: 0 0 0 2px var(--accent); }
</style>
