<script setup>
import { ref } from 'vue'
import { Link, Copy, Check, Trash2, Save, Lock } from 'lucide-vue-next'
import BaseModal from './BaseModal.vue'

const props = defineProps({ share: Object })
const emit = defineEmits(['close', 'updated', 'deleted'])

const copied = ref(false)
const editEditable = ref(props.share.editable)
const editPublic = ref(props.share.public)
const editPassword = ref('')
const editClearPassword = ref(false)
const saving = ref(false)
const error = ref('')

async function copyUrl() {
  await navigator.clipboard.writeText(props.share.url)
  copied.value = true
  setTimeout(() => { copied.value = false }, 2000)
}

async function save() {
  saving.value = true
  error.value = ''
  try {
    const body = {
      editable: editEditable.value,
      public: editPublic.value,
      clear_password: editClearPassword.value,
      password: !editClearPassword.value && editPassword.value ? editPassword.value : undefined,
    }
    const res = await fetch(`/api/shares/${props.share.token}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (res.ok) {
      emit('updated', await res.json())
    } else {
      const data = await res.json().catch(() => ({}))
      error.value = data.detail || 'Failed to save'
    }
  } catch {
    error.value = 'Connection error'
  } finally {
    saving.value = false
  }
}

async function deleteShare() {
  saving.value = true
  try {
    const res = await fetch(`/api/shares/${props.share.token}`, { method: 'DELETE' })
    if (res.ok) emit('deleted')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <BaseModal @close="emit('close')">
    <div class="modal-header">
      <div class="header-text">
        <p class="modal-title">Share properties</p>
      </div>
    </div>

    <div class="modal-body">
      <div class="url-row">
        <Link class="url-icon" />
        <span class="url-text">{{ share.url }}</span>
        <button class="copy-btn" :class="{ copied }" @click="copyUrl">
          <Check v-if="copied" class="copy-icon" />
          <Copy v-else class="copy-icon" />
        </button>
      </div>

      <div class="badges-row">
        <span v-if="share.has_password" class="badge">
          <Lock class="badge-icon" /> Password
        </span>
        <span class="badge" :class="editEditable ? 'badge-edit' : 'badge-ro'">
          {{ editEditable ? 'Editable' : 'Read-only' }}
        </span>
        <span v-if="editPublic" class="badge badge-public">Listed</span>
      </div>

      <div class="field-group">
        <label class="toggle-row">
          <input v-model="editEditable" type="checkbox" class="toggle-cb" />
          <span class="field-label">Allow editing</span>
        </label>
        <label class="toggle-row">
          <input v-model="editPublic" type="checkbox" class="toggle-cb" />
          <span class="field-label">Show on public page</span>
        </label>
        <label class="toggle-row">
          <input v-model="editClearPassword" type="checkbox" class="toggle-cb" />
          <span class="field-label">Remove password</span>
        </label>
        <input
          v-if="!editClearPassword"
          v-model="editPassword"
          class="modal-input"
          type="password"
          placeholder="New password (leave blank to keep)"
          autocomplete="new-password"
        />
      </div>

      <p v-if="error" class="err-msg">{{ error }}</p>
    </div>

    <div class="modal-footer">
      <button class="btn-danger" :disabled="saving" @click="deleteShare">
        <Trash2 style="width:14px;height:14px" />
        Delete link
      </button>
      <div class="footer-right">
        <button class="btn-cancel" @click="emit('close')">Cancel</button>
        <button class="btn-primary" :disabled="saving" @click="save">
          <Save style="width:12px;height:12px" />
          {{ saving ? 'Saving…' : 'Save' }}
        </button>
      </div>
    </div>
  </BaseModal>
</template>

<style scoped>
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
.url-text { font-size: 0.8rem; color: #ababab; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.copy-btn {
  display: flex; align-items: center; justify-content: center;
  width: 28px; height: 28px; border-radius: 6px;
  background: transparent; border: none; color: #636366; flex-shrink: 0;
  transition: background 0.12s, color 0.12s;
}
.copy-btn:hover { background: rgba(255,255,255,0.07); color: #fff; }
.copy-btn.copied { color: #30d158; }
.copy-icon { width: 14px; height: 14px; }

.badges-row { display: flex; gap: 0.35rem; flex-wrap: wrap; }
.badge {
  display: inline-flex; align-items: center; gap: 0.2rem;
  padding: 2px 7px; border-radius: 4px; font-size: 0.7rem; font-weight: 500;
  background: rgba(255,255,255,0.07); color: #636366;
}
.badge-icon { width: 10px; height: 10px; }
.badge-edit { color: #30d158; background: rgba(48,209,88,0.1); }
.badge-ro { color: #636366; }
.badge-public { color: #ff9f0a; background: rgba(255,159,10,0.1); }

.field-group { display: flex; flex-direction: column; gap: 0.45rem; width: 100%; }
.toggle-row { display: flex; align-items: center; gap: 0.5rem; width: 100%; }
.toggle-cb { accent-color: #007AFF; width: 16px; height: 16px; flex-shrink: 0; }
.field-label { font-size: 0.82rem; color: #ababab; }

.modal-input {
  width: 100%; padding: 7px 10px;
  background: rgba(255,255,255,0.05); border: 0.5px solid rgba(255,255,255,0.1);
  border-radius: 7px; color: #fff; font-size: 0.82rem; font-family: inherit; outline: none;
}
.modal-input:focus { border-color: #007AFF; }

.err-msg { font-size: 0.8rem; color: #ff453a; text-align: center; }

.footer-right { display: flex; gap: 0.4rem; align-items: center; }
.btn-danger { display: flex; align-items: center; gap: 0.4rem; }
</style>
