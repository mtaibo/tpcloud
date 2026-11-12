<script setup>
import { alertState, answer, toastState } from './dialog.js'
import Icon from './Icon.vue'
</script>

<template>
  <Teleport to="body">
    <Transition name="alert">
      <div v-if="alertState.open" class="overlay" style="align-items: center; z-index: 300" @click.self="answer(false)">
        <div class="alert" role="alertdialog" aria-modal="true">
          <h2>{{ alertState.title }}</h2>
          <p v-if="alertState.message" class="secondary">{{ alertState.message }}</p>
          <div class="alert-actions" :class="{ pair: alertState.actions.length === 2 }">
            <button
              v-for="a in alertState.actions"
              :key="a.label"
              type="button"
              class="btn"
              :class="a.role === 'cancel' ? 'btn-cancel' : a.role === 'destructive' ? 'btn-prominent destructive' : 'btn-prominent'"
              @click="answer(a.value)"
            >{{ a.label }}</button>
          </div>
        </div>
      </div>
    </Transition>
    <Transition name="toast">
      <div v-if="toastState.visible" class="toast glass" role="status">
        <Icon :name="toastState.icon" :size="20" style="color: var(--green)" />
        {{ toastState.text }}
      </div>
    </Transition>
  </Teleport>
</template>
