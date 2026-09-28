<script setup>
import { ref, computed, onMounted } from 'vue'
import { Lock, Award } from 'lucide-vue-next'
import ExtensionSetupModal from '../components/ExtensionSetupModal.vue'

defineProps({ player: Object, linkedAccount: Object, isOwnAccount: Boolean, user: Object })

const data = ref(null)
const loading = ref(true)
const showCredentials = ref(false)

async function load() {
  loading.value = true
  try {
    const r = await fetch('/api/val/battle-pass')
    if (r.ok) data.value = await r.json()
  } finally { loading.value = false }
}

const active = computed(() => data.value?.contracts?.find(c => c.is_active_battle_pass))
const others = computed(() => data.value?.contracts?.filter(c => !c.is_active_battle_pass) ?? [])
const progressPct = computed(() => {
  if (!active.value) return 0
  return Math.min(100, Math.round((active.value.progression / 5000) * 100))
})

function onCredentialsSaved() { showCredentials.value = false; load() }

onMounted(load)
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Battle Pass</h1>
    </div>

    <div v-if="!loading && data?.requires_credentials" class="cta-card">
      <Lock :size="28" color="#FF4655" />
      <h3 class="cta-title">Conecta tu cuenta Riot</h3>
      <button class="btn-primary" @click="showCredentials = true">Connect</button>
    </div>

    <div v-else-if="active" class="active-card">
      <div class="active-header">
        <Award :size="28" color="#FF4655" />
        <div>
          <p class="active-name">{{ active.name }}</p>
          <p class="active-sub">Nivel {{ active.level }} · {{ active.chapters }} capítulos</p>
        </div>
      </div>
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: progressPct + '%' }" />
      </div>
      <p class="progress-text">{{ active.progression.toLocaleString() }} / 5,000 XP</p>
    </div>

    <section v-if="others.length" class="others">
      <h2 class="section-title">Otros contratos activos</h2>
      <div class="rows">
        <div v-for="c in others" :key="c.uuid" class="row">
          <span class="contract-name">{{ c.name }}</span>
          <span class="contract-level">Nvl {{ c.level }}</span>
        </div>
      </div>
    </section>

    <ExtensionSetupModal v-if="showCredentials" @close="showCredentials = false" @saved="onCredentialsSaved" />
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }
.view-header { padding: 32px 32px 24px; }
.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }

.active-card {
  margin: 0 32px 20px;
  padding: 24px;
  background: rgba(255,70,85,0.05);
  border: 0.5px solid rgba(255,70,85,0.2);
  border-radius: 14px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.active-header { display: flex; align-items: center; gap: 14px; }
.active-name { font-size: 1.1rem; font-weight: 700; color: #fff; }
.active-sub { font-size: 0.78rem; color: #8E8E93; }
.progress-bar { height: 8px; background: rgba(255,255,255,0.08); border-radius: 4px; overflow: hidden; }
.progress-fill { height: 100%; background: #FF4655; transition: width 0.3s; }
.progress-text { font-size: 0.75rem; color: #8E8E93; font-variant-numeric: tabular-nums; }

.section-title { font-size: 0.85rem; font-weight: 600; color: #8E8E93; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px; }
.others { padding: 0 32px 32px; }
.rows { display: flex; flex-direction: column; gap: 6px; }
.row { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; background: rgba(255,255,255,0.03); border-radius: 8px; }
.contract-name { font-size: 0.85rem; color: #d1d1d6; }
.contract-level { font-size: 0.75rem; color: #636366; }

.cta-card { margin: 0 32px; padding: 40px; border-radius: 14px; background: rgba(255,255,255,0.03); display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; }
.cta-title { font-size: 1.125rem; font-weight: 600; }
.btn-primary { padding: 10px 22px; border-radius: 8px; background: #FF4655; border: none; color: #fff; font-size: 0.82rem; font-weight: 700; cursor: pointer; }
</style>
