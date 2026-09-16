<script setup>
import { ref, onMounted } from 'vue'
import { Share2, Copy, Check, Trash2, ChevronRight, Lock, Pencil, Save } from 'lucide-vue-next'

const emit = defineEmits(['navigate'])

const shares = ref([])
const loading = ref(true)
const copiedToken = ref(null)
const editingToken = ref(null)
const editPassword = ref('')
const editClearPassword = ref(false)
const editEditable = ref(false)
const saving = ref(false)

async function load() {
  loading.value = true
  try {
    const res = await fetch('/api/shares')
    if (res.ok) shares.value = await res.json()
  } finally {
    loading.value = false
  }
}

async function copyUrl(share) {
  await navigator.clipboard.writeText(share.url)
  copiedToken.value = share.token
  setTimeout(() => { copiedToken.value = null }, 2000)
}

const editPublic = ref(false)

function startEdit(share) {
  editingToken.value = share.token
  editEditable.value = share.editable
  editPublic.value = share.public
  editPassword.value = ''
  editClearPassword.value = false
}

function cancelEdit() {
  editingToken.value = null
}

async function saveEdit(share) {
  saving.value = true
  try {
    const body = {
      editable: editEditable.value,
      public: editPublic.value,
      clear_password: editClearPassword.value,
      password: !editClearPassword.value && editPassword.value ? editPassword.value : undefined,
    }
    const res = await fetch(`/api/shares/${share.token}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    if (res.ok) {
      const updated = await res.json()
      const idx = shares.value.findIndex(s => s.token === share.token)
      if (idx !== -1) shares.value[idx] = updated
      editingToken.value = null
    }
  } finally {
    saving.value = false
  }
}

async function deleteShare(share) {
  const res = await fetch(`/api/shares/${share.token}`, { method: 'DELETE' })
  if (res.ok) shares.value = shares.value.filter(s => s.token !== share.token)
}

function folderName(share) {
  const parts = share.path.split('/').filter(Boolean)
  return parts.at(-1) || share.token
}

function goToFolder(share) {
  emit('navigate', share.location, share.path)
}

onMounted(load)
</script>

<template>
  <div class="panel">
    <div class="panel-topbar">
      <Share2 class="panel-icon" />
      <span class="panel-title">Shared Links</span>
    </div>

    <div v-if="loading" class="state-msg">Loading…</div>
    <div v-else-if="!shares.length" class="state-msg">No shared links yet.<br>Right-click a folder to share it.</div>

    <div v-else class="shares-list">
      <div v-for="share in shares" :key="share.token" class="share-card">
        <div class="share-main">
          <div class="share-info">
            <div class="share-name-row">
              <span class="share-name">{{ folderName(share) }}</span>
              <span v-if="share.has_password" class="badge">
                <Lock class="badge-icon" /> Password
              </span>
              <span class="badge" :class="share.editable ? 'badge-edit' : 'badge-ro'">
                {{ share.editable ? 'Editable' : 'Read-only' }}
              </span>
              <span v-if="share.public" class="badge badge-public">Listed</span>
            </div>
            <span class="share-path">{{ share.path }}</span>
            <div class="url-row">
              <span class="share-url">{{ share.url }}</span>
            </div>
          </div>

          <div class="share-actions">
            <button class="action-btn" :class="{ done: copiedToken === share.token }" title="Copy link" @click="copyUrl(share)">
              <Check v-if="copiedToken === share.token" class="action-icon" />
              <Copy v-else class="action-icon" />
            </button>
            <button class="action-btn" title="Go to folder" @click="goToFolder(share)">
              <ChevronRight class="action-icon" />
            </button>
            <button class="action-btn" title="Edit" @click="editingToken === share.token ? cancelEdit() : startEdit(share)">
              <Pencil class="action-icon" />
            </button>
            <button class="action-btn action-btn--danger" title="Delete" @click="deleteShare(share)">
              <Trash2 class="action-icon" />
            </button>
          </div>
        </div>

        <!-- Inline edit -->
        <div v-if="editingToken === share.token" class="edit-form">
          <label class="edit-row">
            <input v-model="editEditable" type="checkbox" class="edit-cb" />
            <span class="edit-label">Allow editing</span>
          </label>
          <label class="edit-row">
            <input v-model="editPublic" type="checkbox" class="edit-cb" />
            <span class="edit-label">Show on public page</span>
          </label>

          <div class="edit-row">
            <label class="edit-row" style="flex:1">
              <input v-model="editClearPassword" type="checkbox" class="edit-cb" />
              <span class="edit-label">Remove password</span>
            </label>
          </div>

          <input
            v-if="!editClearPassword"
            v-model="editPassword"
            class="edit-input"
            type="password"
            placeholder="New password (leave blank to keep)"
            autocomplete="new-password"
          />

          <div class="edit-footer">
            <button class="btn-cancel-sm" @click="cancelEdit">Cancel</button>
            <button class="btn-save" :disabled="saving" @click="saveEdit(share)">
              <Save class="btn-icon" />
              {{ saving ? 'Saving…' : 'Save' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-topbar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 14px 1.25rem;
  border-bottom: 0.5px solid rgba(255,255,255,0.08);
}

.panel-icon { width: 18px; height: 18px; color: #007AFF; flex-shrink: 0; }
.panel-title { font-size: 0.95rem; font-weight: 600; color: #fff; }

.state-msg {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  font-size: 0.875rem;
  color: #636366;
  line-height: 1.6;
  padding: 4rem 2rem;
}

.shares-list {
  flex: 1;
  overflow-y: auto;
  padding: 1rem 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.share-card {
  background: rgba(255,255,255,0.03);
  border: 0.5px solid rgba(255,255,255,0.08);
  border-radius: 10px;
  overflow: hidden;
}

.share-main {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 0.8rem 0.9rem;
}

.share-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.share-name-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  flex-wrap: wrap;
}

.share-name {
  font-size: 0.9rem;
  font-weight: 600;
  color: #e4e4e7;
}

.badge {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 0.7rem;
  font-weight: 500;
  background: rgba(255,255,255,0.07);
  color: #636366;
}
.badge-icon { width: 10px; height: 10px; }
.badge-edit { color: #30d158; background: rgba(48,209,88,0.1); }
.badge-ro { color: #636366; }
.badge-public { color: #ff9f0a; background: rgba(255,159,10,0.1); }

.share-path {
  font-size: 0.75rem;
  color: #3a3a3c;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.url-row { margin-top: 0.1rem; }

.share-url {
  font-size: 0.78rem;
  color: #007AFF;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: block;
}

.share-actions {
  display: flex;
  align-items: center;
  gap: 0.2rem;
  flex-shrink: 0;
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 7px;
  background: transparent;
  border: none;
  color: #636366;
  transition: background 0.12s, color 0.12s;
}
.action-btn:hover { background: rgba(255,255,255,0.07); color: #fff; }
.action-btn.done { color: #30d158; }
.action-btn--danger:hover { background: rgba(255,69,58,0.12); color: #ff453a; }
.action-icon { width: 14px; height: 14px; }

/* Inline edit form */
.edit-form {
  padding: 0.75rem 0.9rem;
  border-top: 0.5px solid rgba(255,255,255,0.07);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  background: rgba(0,0,0,0.2);
}

.edit-row {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.edit-cb { accent-color: #007AFF; width: 14px; height: 14px; flex-shrink: 0; }

.edit-label { font-size: 0.82rem; color: #ababab; }

.edit-input {
  width: 100%;
  padding: 7px 10px;
  background: rgba(255,255,255,0.05);
  border: 0.5px solid rgba(255,255,255,0.1);
  border-radius: 7px;
  color: #fff;
  font-size: 0.82rem;
  font-family: inherit;
  outline: none;
}
.edit-input:focus { border-color: #007AFF; }

.edit-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.4rem;
  margin-top: 0.25rem;
}

.btn-cancel-sm {
  padding: 5px 12px;
  border-radius: 7px;
  background: transparent;
  border: 0.5px solid rgba(255,255,255,0.1);
  color: #636366;
  font-size: 0.82rem;
  font-family: inherit;
  transition: background 0.12s;
}
.btn-cancel-sm:hover { background: rgba(255,255,255,0.06); color: #fff; }

.btn-save {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding: 5px 12px;
  border-radius: 7px;
  background: #007AFF;
  border: none;
  color: #fff;
  font-size: 0.82rem;
  font-weight: 600;
  font-family: inherit;
  transition: opacity 0.12s;
}
.btn-save:disabled { opacity: 0.5; }
.btn-icon { width: 12px; height: 12px; }

@media (max-width: 767px) {
  .shares-list { padding: 0.75rem; }
  .panel-topbar { padding-top: calc(14px + env(safe-area-inset-top)); }
}
</style>
