<script setup>
import { ref, onMounted } from 'vue'
import { Lock, Wallet, Coins, Crown } from 'lucide-vue-next'
import ExtensionSetupModal from '../components/ExtensionSetupModal.vue'

defineProps({ player: Object, linkedAccount: Object, isOwnAccount: Boolean, user: Object })

const wallet = ref(null)
const loading = ref(true)
const error = ref('')
const showCredentials = ref(false)

async function load() {
  loading.value = true
  error.value = ''
  try {
    const r = await fetch('/api/val/wallet')
    if (r.ok) wallet.value = await r.json()
    else error.value = 'Failed to load wallet'
  } catch { error.value = 'Network error' } finally { loading.value = false }
}

function onCredentialsSaved() { showCredentials.value = false; load() }

onMounted(load)
</script>

<template>
  <div class="view">
    <div class="view-header">
      <h1 class="view-title">Wallet</h1>
    </div>

    <div v-if="!loading && wallet?.requires_credentials" class="cta-card">
      <Lock :size="28" color="#FF4655" />
      <h3 class="cta-title">Conecta tu cuenta Riot</h3>
      <p class="cta-text">Necesitas la extensión TPVal Sync para ver tu wallet.</p>
      <button class="btn-primary" @click="showCredentials = true">Connect</button>
    </div>

    <div v-else-if="loading" class="cards">
      <div class="card-skeleton" v-for="i in 3" :key="i" />
    </div>

    <div v-else-if="wallet" class="cards">
      <div class="card vp">
        <div class="icon-wrap"><Wallet :size="20" /></div>
        <p class="label">Valorant Points</p>
        <p class="amount">{{ wallet.vp?.toLocaleString() ?? 0 }}</p>
      </div>
      <div class="card rp">
        <div class="icon-wrap"><Coins :size="20" /></div>
        <p class="label">Radianite Points</p>
        <p class="amount">{{ wallet.rp?.toLocaleString() ?? 0 }}</p>
      </div>
      <div class="card kc">
        <div class="icon-wrap"><Crown :size="20" /></div>
        <p class="label">Kingdom Credits</p>
        <p class="amount">{{ wallet.kc?.toLocaleString() ?? 0 }}</p>
      </div>
    </div>

    <p v-if="error" class="error-banner">{{ error }}</p>

    <ExtensionSetupModal v-if="showCredentials" @close="showCredentials = false" @saved="onCredentialsSaved" />
  </div>
</template>

<style scoped>
.view { flex: 1; overflow-y: auto; }
.view-header { padding: 32px 32px 24px; }
.view-title { font-size: 1.5rem; font-weight: 700; letter-spacing: -0.02em; }

.cards { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; padding: 0 32px 32px; }
.card {
  padding: 20px; border-radius: 14px;
  background: rgba(255,255,255,0.03);
  border: 0.5px solid rgba(255,255,255,0.06);
  display: flex; flex-direction: column; gap: 8px;
}
.icon-wrap { width: 32px; height: 32px; border-radius: 8px; display: flex; align-items: center; justify-content: center; }
.card.vp .icon-wrap { background: rgba(255, 70, 85, 0.14); color: #FF4655; }
.card.rp .icon-wrap { background: rgba(52, 199, 89, 0.14); color: #30d158; }
.card.kc .icon-wrap { background: rgba(255, 214, 10, 0.14); color: #ffd60a; }
.label { font-size: 0.75rem; color: #8E8E93; text-transform: uppercase; letter-spacing: 0.05em; }
.amount { font-size: 1.5rem; font-weight: 700; color: #fff; font-variant-numeric: tabular-nums; }

.card-skeleton { height: 108px; border-radius: 14px; background: rgba(255,255,255,0.04); animation: pulse 1.5s infinite; }
@keyframes pulse { 0%,100%{opacity:1} 50%{opacity:.5} }

.cta-card { margin: 0 32px; padding: 40px; border-radius: 14px; background: rgba(255,255,255,0.03); border: 0.5px solid rgba(255,255,255,0.06); display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; }
.cta-title { font-size: 1.125rem; font-weight: 600; }
.cta-text { font-size: 0.875rem; color: #636366; max-width: 360px; line-height: 1.5; }
.btn-primary { padding: 10px 22px; border-radius: 8px; background: #FF4655; border: none; color: #fff; font-size: 0.82rem; font-weight: 700; cursor: pointer; }
.error-banner { padding: 20px 32px; font-size: 0.875rem; color: #ff453a; }

@media (max-width: 767px) { .cards { grid-template-columns: 1fr; padding: 0 16px 16px; } }
</style>
