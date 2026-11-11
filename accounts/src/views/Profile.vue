<script setup>
import { computed, ref, watch } from 'vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import { api } from '@ui/api.js'
import { toast } from '@ui/dialog.js'
import { initials } from '@ui/format.js'
import Page from '../components/Page.vue'
import { store } from '../store.js'

const profile = computed(() => store.profile)
const name = ref(profile.value.display_name)
watch(() => profile.value.display_name, value => { name.value = value })

async function saveName(e) {
  const value = name.value.trim()
  if (!value) { name.value = profile.value.display_name; return }
  if (value === profile.value.display_name) return
  try {
    await api('/auth/account/profile', { method: 'PATCH', body: { display_name: value } })
    await store.reload()
    toast('Nombre actualizado')
  } catch (err) {
    name.value = profile.value.display_name
    toast(err.message, 'warning')
  }
  e?.target?.blur?.()
}
</script>

<template>
  <Page title="Perfil">
    <template #header>
      <div class="hero">
        <span class="avatar" style="width: 96px; height: 96px; font-size: 38px">{{ initials(profile.display_name) }}</span>
        <h1 class="title-1">{{ profile.display_name }}</h1>
        <p class="subheadline secondary">{{ profile.email }}</p>
      </div>
    </template>

    <ListSection footer="Así aparece tu nombre en TPCloud y en sus módulos.">
      <ListRow>
        <span class="row-title row-label">Nombre</span>
        <template #trailing>
          <input
            v-model="name"
            class="row-input trailing"
            autocomplete="name"
            aria-label="Nombre"
            @keydown.enter="saveName"
            @blur="saveName"
          />
        </template>
      </ListRow>
      <ListRow title="Correo" :value="profile.email" />
      <ListRow title="Cuenta" :value="profile.is_admin ? 'Administrador' : 'Usuario'" />
    </ListSection>
  </Page>
</template>

<style scoped>
.hero { display: flex; flex-direction: column; align-items: center; gap: 6px; text-align: center; padding: 8px 0 8px; }
.hero h1 { margin-top: 10px; overflow-wrap: anywhere; }
.row-label { flex: 0 0 auto; width: auto; }
</style>
