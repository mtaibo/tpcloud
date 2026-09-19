<script setup>
import { ref, onMounted } from 'vue'
import { Folder, Lock } from 'lucide-vue-next'

const shares = ref([])
const loading = ref(true)

onMounted(async () => {
  try {
    const res = await fetch('/api/share')
    if (res.ok) shares.value = await res.json()
  } finally {
    loading.value = false
  }
})

function open(token) {
  window.location.href = '/' + token
}
</script>

<template>
  <div class="landing">
    <header class="landing-header">
      <h1 class="site-title">TPShare</h1>
    </header>

    <main class="landing-main">
      <div v-if="loading" class="state-msg">Loading…</div>
      <div v-else-if="!shares.length" class="state-msg">Nothing shared publicly yet.</div>

      <div v-else class="grid">
        <button
          v-for="share in shares"
          :key="share.token"
          class="card"
          @click="open(share.token)"
        >
          <div class="card-icon">
            <Folder class="folder-icon" />
          </div>
          <div class="card-body">
            <span class="card-name">{{ share.label }}</span>
            <span v-if="share.has_password" class="card-lock">
              <Lock class="lock-icon" /> Password protected
            </span>
          </div>
        </button>
      </div>
    </main>
  </div>
</template>

<style scoped>
.landing {
  min-height: 100dvh;
  display: flex;
  flex-direction: column;
  padding: 0 1.5rem;
  max-width: 720px;
  margin: 0 auto;
  width: 100%;
}

.landing-header {
  padding: 3rem 0 2rem;
}

.site-title {
  font-size: 2rem;
  font-weight: 700;
  color: #fff;
  letter-spacing: -0.03em;
}

.landing-main { flex: 1; }

.state-msg {
  font-size: 0.9rem;
  color: #636366;
  padding: 2rem 0;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 0.75rem;
}

.card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 1.1rem;
  background: rgba(28, 28, 30, 0.8);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  border-radius: 14px;
  text-align: left;
  transition: background 0.15s, border-color 0.15s, transform 0.12s;
  width: 100%;
}

.card:hover {
  background: rgba(40, 40, 42, 0.95);
  border-color: rgba(255, 255, 255, 0.18);
  transform: translateY(-1px);
}

.card:active { transform: translateY(0); }

.card-icon {
  width: 44px;
  height: 44px;
  border-radius: 11px;
  background: rgba(142, 142, 147, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
}

.folder-icon { width: 22px; height: 22px; color: #8E8E93; }

.card-body {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  min-width: 0;
  width: 100%;
}

.card-name {
  font-size: 0.95rem;
  font-weight: 600;
  color: #e4e4e7;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-lock {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.72rem;
  color: #636366;
}

.lock-icon { width: 11px; height: 11px; }

@media (max-width: 480px) {
  .grid { grid-template-columns: repeat(2, 1fr); }
  .landing-header { padding: 2rem 0 1.5rem; }
  .site-title { font-size: 1.5rem; }
}
</style>
