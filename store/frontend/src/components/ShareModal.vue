<script setup>
import { ref, computed, onMounted } from 'vue'
import { Share2, Copy, Check, Trash2, Link } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const props = defineProps({
  entry: Object,
  location: String,
  currentPath: String,
})
const emit = defineEmits(['close'])

const loading = ref(true)
const existingShare = ref(null)
const copied = ref(false)
const error = ref('')
const saving = ref(false)

// Creation form
const slugMode = ref('custom')
const slugValue = ref('')
const usePassword = ref(false)
const passwordValue = ref('')
const editable = ref(false)

const entryPath = computed(() =>
  props.currentPath ? `${props.currentPath}/${props.entry.name}` : props.entry.name
)

const shareUrl = computed(() => {
  if (!existingShare.value) return ''
  return `https://share.migueltaibo.com/${existingShare.value.token}`
})

onMounted(async () => {
  try {
    const params = new URLSearchParams({ path: entryPath.value, location: props.location })
    const res = await fetch(`/api/shares?${params}`)
    if (res.ok) {
      const data = await res.json()
      existingShare.value = data[0] ?? null
    }
  } catch {}
  slugValue.value = props.entry.name.toLowerCase().replace(/[^a-z0-9-]/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '')
  loading.value = false
})

async function createShare() {
  error.value = ''
  saving.value = true
  try {
    const body = {
      location: props.location,
      path: entryPath.value,
      hidden: slugMode.value === 'hidden',
      slug: slugMode.value === 'custom' ? slugValue.value : undefined,
      password: usePassword.value && passwordValue.value ? passwordValue.value : undefined,
      editable: editable.value,
    }
    const res = await fetch('/api/shares', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (!res.ok) {
      const data = await res.json().catch(() => ({}))
      error.value = data.detail || 'Failed to create share'
      return
    }
    existingShare.value = await res.json()
  } catch {
    error.value = 'Connection error'
  } finally {
    saving.value = false
  }
}

async function deleteShare() {
  saving.value = true
  try {
    const res = await fetch(`/api/shares/${existingShare.value.token}`, { method: 'DELETE' })
    if (res.ok) existingShare.value = null
  } catch {}
  saving.value = false
}

async function copyUrl() {
  await navigator.clipboard.writeText(shareUrl.value)
  copied.value = true
  setTimeout(() => { copied.value = false }, 2000)
}
</script>

<template>
  <BaseModal @close="emit('close')">
    <div class="modal-header">
      <div class="entry-icon" style="color: #007AFF">
        <Share2 style="width:18px;height:18px" />
      </div>
      <div class="header-text">
        <p class="modal-title">Share "{{ entry.name }}"</p>
      </div>
    </div>

    <div class="modal-body">
      <div v-if="loading" class="state-msg">Loading…</div>

      <!-- Share already exists -->
      <template v-else-if="existingShare">
        <div class="url-row">
          <Link class="url-icon" />
          <span class="url-text">{{ shareUrl }}</span>
          <button class="copy-btn" :class="{ copied }" @click="copyUrl">
            <Check v-if="copied" class="copy-icon" />
            <Copy v-else class="copy-icon" />
          </button>
        </div>
        <p class="share-meta">
          {{ existingShare.editable ? 'Editable' : 'Read-only' }}
          · {{ existingShare.has_password ? 'Password protected' : 'Public' }}
        </p>
      </template>

      <!-- Creation form -->
      <template v-else>
        <div class="field-group">
          <label class="field-label">Link name</label>
          <div class="radio-group">
            <label class="radio-opt" :class="{ selected: slugMode === 'custom' }">
              <input v-model="slugMode" type="radio" value="custom" />
              Custom
            </label>
            <label class="radio-opt" :class="{ selected: slugMode === 'hidden' }">
              <input v-model="slugMode" type="radio" value="hidden" />
              Random (hidden)
            </label>
          </div>
          <input
            v-if="slugMode === 'custom'"
            v-model="slugValue"
            class="modal-input"
            placeholder="e.g. mallorca"
            spellcheck="false"
            autocomplete="off"
          />
        </div>

        <div class="field-group">
          <label class="toggle-row">
            <input v-model="usePassword" type="checkbox" class="toggle-cb" />
            <span class="field-label" style="margin:0">Password protect</span>
          </label>
          <input
            v-if="usePassword"
            v-model="passwordValue"
            class="modal-input"
            type="password"
            placeholder="Password"
            autocomplete="new-password"
          />
        </div>

        <label class="toggle-row">
          <input v-model="editable" type="checkbox" class="toggle-cb" />
          <span class="field-label" style="margin:0">Allow editing</span>
        </label>

        <p v-if="error" class="err-msg">{{ error }}</p>
      </template>
    </div>

    <div class="modal-footer">
      <template v-if="existingShare">
        <button class="btn-danger" :disabled="saving" @click="deleteShare">
          <Trash2 style="width:14px;height:14px" />
          Delete link
        </button>
        <button class="btn-cancel" @click="emit('close')">Done</button>
      </template>
      <template v-else-if="!loading">
        <button class="btn-cancel" @click="emit('close')">Cancel</button>
        <button class="btn-primary" :disabled="saving || (slugMode === 'custom' && !slugValue)" @click="createShare">
          {{ saving ? 'Creating…' : 'Create link' }}
        </button>
      </template>
    </div>
  </BaseModal>
</template>

<style scoped>
.state-msg { font-size: 0.85rem; color: #636366; text-align: center; padding: 1rem 0; }

.url-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 8px 10px;
  background: rgba(255, 255, 255, 0.05);
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
}

.url-icon { width: 14px; height: 14px; color: #636366; flex-shrink: 0; }

.url-text {
  font-size: 0.8rem;
  color: #ababab;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.copy-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: #636366;
  flex-shrink: 0;
  transition: background 0.12s, color 0.12s;
}
.copy-btn:hover { background: rgba(255,255,255,0.07); color: #fff; }
.copy-btn.copied { color: #30d158; }
.copy-icon { width: 14px; height: 14px; }

.share-meta {
  font-size: 0.75rem;
  color: #3a3a3c;
  text-align: center;
}

.field-group { display: flex; flex-direction: column; gap: 0.4rem; width: 100%; }

.field-label {
  font-size: 0.75rem;
  color: #636366;
  font-weight: 500;
  letter-spacing: 0.02em;
  margin-bottom: 0.1rem;
}

.radio-group { display: flex; gap: 0.5rem; }

.radio-opt {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.85rem;
  color: #ababab;
  padding: 5px 10px;
  border-radius: 6px;
  background: rgba(255,255,255,0.04);
  border: 0.5px solid rgba(255,255,255,0.08);
  transition: background 0.12s;
}
.radio-opt.selected { background: rgba(0,122,255,0.15); border-color: rgba(0,122,255,0.3); color: #fff; }
.radio-opt input { display: none; }

.toggle-row { display: flex; align-items: center; gap: 0.5rem; width: 100%; }

.toggle-cb { accent-color: #007AFF; width: 16px; height: 16px; flex-shrink: 0; }

.err-msg { font-size: 0.8rem; color: #ff453a; text-align: center; }

.btn-danger {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
</style>
