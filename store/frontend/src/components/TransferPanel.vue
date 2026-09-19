<script setup>
import { watch } from 'vue'
import { Check, X, ArrowUp, ArrowDown, Play, Square } from 'lucide-vue-next'
import { useTransfers, formatBytes, formatEta } from '../useTransfers.js'

const props = defineProps({ show: Boolean })

const { uploads, downloads, pendingServerUploads, cancelTransfer, resumeUpload, fetchPendingUploads } = useTransfers()

watch(() => props.show, val => { if (val) fetchPendingUploads() })

function progress(t) {
  return t.totalSize > 0 ? Math.min(100, (t.loaded / t.totalSize) * 100) : null
}

function eta(t) {
  if (t.status !== 'active' || !t.speed || t.speed <= 0 || !t.totalSize) return null
  return formatEta((t.totalSize - t.loaded) / t.speed)
}

function speed(t) {
  return t.status === 'active' && t.speed > 0 ? `${formatBytes(t.speed)}/s` : null
}

function pendingPct(p) {
  return p.file_size > 0 ? Math.round((p.bytes_received / p.file_size) * 100) : 0
}
</script>

<template>
  <Teleport to="body">
    <Transition name="panel">
      <div v-if="show" class="tp">

        <!-- Active uploads -->
        <section v-if="uploads.length" class="sec">
          <div class="sec-head">
            <ArrowUp class="sec-icon" />
            <span>Uploads</span>
            <span class="badge">{{ uploads.length }}</span>
          </div>
          <div v-for="t in uploads" :key="t.id" class="item">
            <div class="item-top">
              <div class="item-name" :title="t.name">{{ t.name }}</div>
              <button class="cancel-btn" title="Cancel" @click="cancelTransfer(t.id)">
                <Square class="cancel-icon" />
              </button>
            </div>
            <div class="bar-wrap">
              <div
                class="bar"
                :class="{ 'bar--done': t.status === 'done', 'bar--error': t.status === 'error', 'bar--indet': !t.totalSize && t.status === 'active' }"
                :style="t.totalSize ? { width: progress(t) + '%' } : {}"
              />
            </div>
            <div class="item-meta">
              <template v-if="t.status === 'active'">
                {{ formatBytes(t.loaded) }}{{ t.totalSize ? ` / ${formatBytes(t.totalSize)}` : '' }}<template v-if="speed(t)"> · {{ speed(t) }}</template><template v-if="eta(t)"> · {{ eta(t) }} left</template>
              </template>
              <span v-else-if="t.status === 'done'" class="done-label"><Check class="si" />{{ formatBytes(t.totalSize || t.loaded) }}</span>
              <span v-else class="err-label"><X class="si" />{{ t.error }}</span>
            </div>
          </div>
        </section>

        <!-- Paused / resumable uploads -->
        <section v-if="pendingServerUploads.length" class="sec">
          <div v-if="uploads.length" class="inner-sep" />
          <div class="sec-head">
            <ArrowUp class="sec-icon paused-icon" />
            <span>Paused</span>
            <span class="badge">{{ pendingServerUploads.length }}</span>
          </div>
          <div v-for="p in pendingServerUploads" :key="p.upload_id" class="item">
            <div class="item-top">
              <div class="item-name" :title="p.filename">{{ p.filename }}</div>
              <button class="resume-btn" title="Resume" @click="resumeUpload(p)">
                <Play class="resume-icon" />
              </button>
            </div>
            <div class="bar-wrap">
              <div class="bar bar--paused" :style="{ width: pendingPct(p) + '%' }" />
            </div>
            <div class="item-meta">
              {{ formatBytes(p.bytes_received) }} / {{ formatBytes(p.file_size) }} · {{ pendingPct(p) }}%
            </div>
          </div>
        </section>

        <div v-if="(uploads.length || pendingServerUploads.length) && downloads.length" class="sep" />

        <!-- Active downloads -->
        <section v-if="downloads.length" class="sec">
          <div class="sec-head">
            <ArrowDown class="sec-icon" />
            <span>Downloads</span>
            <span class="badge">{{ downloads.length }}</span>
          </div>
          <div v-for="t in downloads" :key="t.id" class="item">
            <div class="item-top">
              <div class="item-name" :title="t.name">{{ t.name }}</div>
              <button class="cancel-btn" title="Cancel" @click="cancelTransfer(t.id)">
                <Square class="cancel-icon" />
              </button>
            </div>
            <div class="bar-wrap">
              <div
                class="bar bar--dl"
                :class="{ 'bar--done': t.status === 'done', 'bar--error': t.status === 'error', 'bar--indet': !t.totalSize && t.status === 'active' }"
                :style="t.totalSize ? { width: progress(t) + '%' } : {}"
              />
            </div>
            <div class="item-meta">
              <template v-if="t.status === 'active'">
                {{ formatBytes(t.loaded) }}{{ t.totalSize ? ` / ${formatBytes(t.totalSize)}` : '' }}<template v-if="speed(t)"> · {{ speed(t) }}</template><template v-if="eta(t)"> · {{ eta(t) }} left</template>
              </template>
              <span v-else-if="t.status === 'done'" class="done-label"><Check class="si" />{{ formatBytes(t.totalSize || t.loaded) }}</span>
              <span v-else class="err-label"><X class="si" />{{ t.error }}</span>
            </div>
          </div>
        </section>

      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.tp {
  position: fixed;
  left: 248px;
  bottom: 72px;
  width: 320px;
  max-height: 60vh;
  overflow-y: auto;
  background: #1c1c1e;
  border: 0.5px solid rgba(255, 255, 255, 0.1);
  border-radius: 14px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  z-index: 1000;
  padding: 0.625rem 0;
}

.sec { padding: 0 0.875rem; }

.sec-head {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0 0.5rem;
  font-size: 0.7rem;
  font-weight: 600;
  letter-spacing: 0.02em;
  color: #636366;
}

.sec-icon { width: 12px; height: 12px; }
.paused-icon { color: #ff9f0a; opacity: 0.8; }

.badge {
  margin-left: auto;
  background: #2c2c2e;
  border-radius: 4px;
  padding: 1px 5px;
  font-size: 0.65rem;
  color: #aeaeb2;
  font-weight: 600;
}

.item {
  padding: 0.5rem 0;
  border-top: 0.5px solid rgba(255, 255, 255, 0.05);
}

.sec > .item:first-of-type { border-top: none; }

.item-top {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  margin-bottom: 0.375rem;
}

.item-name {
  flex: 1;
  font-size: 0.8rem;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}

.cancel-btn, .resume-btn {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  background: none;
  border: none;
  border-radius: 4px;
  cursor: default;
  opacity: 0.4;
  transition: opacity 0.15s;
}

.cancel-btn:hover { opacity: 1; }
.resume-btn { opacity: 0.6; }
.resume-btn:hover { opacity: 1; }

.cancel-icon { width: 12px; height: 12px; color: #ff453a; }
.resume-icon { width: 12px; height: 12px; color: #30d158; }

.bar-wrap {
  height: 3px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 0.375rem;
  position: relative;
}

.bar {
  height: 100%;
  border-radius: 2px;
  background: #0a84ff;
  transition: width 0.4s ease;
  min-width: 0;
}

.bar--dl { background: #30d158; }
.bar--paused { background: #ff9f0a; opacity: 0.7; }
.bar--done { opacity: 0.5; width: 100% !important; }
.bar--error { background: #ff453a !important; width: 100% !important; opacity: 0.7; }

.bar--indet {
  position: absolute;
  width: 35% !important;
  animation: indet 1.5s ease-in-out infinite;
}

@keyframes indet {
  0% { left: -35%; }
  100% { left: 135%; }
}

.item-meta {
  font-size: 0.7rem;
  color: #636366;
  display: flex;
  align-items: center;
  gap: 0.25rem;
  min-height: 1em;
}

.done-label { display: flex; align-items: center; gap: 0.25rem; color: #30d158; }
.err-label { display: flex; align-items: center; gap: 0.25rem; color: #ff453a; }
.si { width: 11px; height: 11px; flex-shrink: 0; }

.sep { height: 0.5px; background: rgba(255, 255, 255, 0.07); margin: 0.375rem 0.875rem; }
.inner-sep { height: 0.5px; background: rgba(255, 255, 255, 0.05); margin: 0.25rem 0; }

.panel-enter-active, .panel-leave-active { transition: opacity 0.18s, transform 0.18s; }
.panel-enter-from, .panel-leave-to { opacity: 0; transform: translateY(6px); }
</style>
