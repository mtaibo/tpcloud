<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import QRCode from 'qrcode'
import Icon from '@ui/Icon.vue'
import Spinner from '@ui/Spinner.vue'

// Shows a QR code another signed-in device scans to let this browser in, and waits for it.
const emit = defineEmits(['approved'])

const state = ref('loading') // loading | pending | scanned | approved | denied | error
const svg = ref('')
const expiresAt = ref(0)
const now = ref(Date.now())
let pollTimer = null
let clockTimer = null
let stopped = false

const remaining = computed(() => Math.max(0, Math.ceil((expiresAt.value - now.value) / 1000)))
const countdown = computed(() => `${Math.floor(remaining.value / 60)}:${String(remaining.value % 60).padStart(2, '0')}`)

async function start() {
  clearTimeout(pollTimer)
  state.value = 'loading'
  try {
    const res = await fetch('/auth/qr/start', { method: 'POST' })
    const data = await res.json()
    if (!res.ok) throw new Error(data.detail)
    svg.value = await QRCode.toString(data.url, {
      type: 'svg',
      errorCorrectionLevel: 'H',
      margin: 0,
      color: { dark: '#000000ff', light: '#00000000' },
    })
    expiresAt.value = Date.now() + data.expires_in * 1000
    state.value = 'pending'
    schedulePoll()
  } catch {
    state.value = 'error'
  }
}

function schedulePoll() {
  if (stopped) return
  pollTimer = setTimeout(poll, 1500)
}

async function poll() {
  if (document.hidden) return schedulePoll()
  try {
    const { status } = await (await fetch('/auth/qr/poll')).json()
    if (status === 'approved') {
      state.value = 'approved'
      emit('approved')
      return
    }
    if (status === 'expired') return start()
    if (status === 'denied') {
      state.value = 'denied'
      return
    }
    state.value = status
  } catch { /* try again on the next tick */ }
  schedulePoll()
}

onMounted(() => {
  start()
  clockTimer = setInterval(() => { now.value = Date.now() }, 1000)
})
onBeforeUnmount(() => {
  stopped = true
  clearTimeout(pollTimer)
  clearInterval(clockTimer)
})
</script>

<template>
  <div class="qr-panel">
    <div class="qr-frame" :class="{ dimmed: state !== 'pending' }">
      <div v-if="svg" class="qr-code" v-html="svg" />
      <span class="qr-mark" aria-hidden="true">
        <svg width="24" height="24" viewBox="0 0 32 32"><rect x="8" y="5.6" width="16" height="4.4" rx="2.2" fill="#fff" /><circle cx="16" cy="16.2" r="3.2" fill="#fff" /><circle cx="16" cy="24.4" r="2.4" fill="#fff" /></svg>
      </span>
      <Transition name="fade">
        <div v-if="state !== 'pending'" class="qr-state">
          <Spinner v-if="state === 'loading'" :size="28" />
          <template v-else-if="state === 'scanned'">
            <span class="state-icon blue"><Icon name="iphone" :size="30" /></span>
            <p class="headline">Confirm on your iPhone</p>
          </template>
          <template v-else-if="state === 'approved'">
            <span class="state-icon green"><Icon name="checkmark" :size="30" :weight="2.6" /></span>
            <p class="headline">Approved</p>
          </template>
          <template v-else-if="state === 'denied' || state === 'error'">
            <span class="state-icon red"><Icon name="xmark" :size="28" :weight="2.6" /></span>
            <p class="headline">{{ state === 'denied' ? 'Declined' : "Couldn't create the code" }}</p>
            <button class="btn btn-bordered small" type="button" @click="start">Get a new one</button>
          </template>
        </div>
      </Transition>
    </div>

    <ol v-if="state !== 'scanned'" class="steps subheadline">
      <li><span>1</span>Open the Camera or the TPCloud app on an iPhone that is signed in.</li>
      <li><span>2</span>Point it at the code.</li>
      <li><span>3</span>Check the device and confirm.</li>
    </ol>
    <p v-else class="subheadline secondary center">On your iPhone, check which device is asking to sign in and from where before you approve it.</p>
    <p v-if="state === 'pending'" class="footnote tertiary center">The code refreshes in {{ countdown }}</p>
  </div>
</template>

<style scoped>
.qr-panel { display: flex; flex-direction: column; align-items: center; gap: 20px; }
.qr-frame {
  position: relative;
  width: 232px;
  height: 232px;
  padding: 18px;
  border-radius: 30px;
  background: #fff;
  box-shadow: 0 0 0 0.5px rgba(0, 0, 0, 0.08), 0 10px 30px rgba(0, 0, 0, 0.08);
}
.qr-code { width: 100%; height: 100%; transition: filter 0.3s, opacity 0.3s; }
.qr-code :deep(svg) { display: block; width: 100%; height: 100%; }
.qr-frame.dimmed .qr-code { filter: blur(6px); opacity: 0.25; }
.qr-mark {
  position: absolute;
  top: 50%;
  left: 50%;
  width: 40px;
  height: 40px;
  margin: -20px 0 0 -20px;
  display: grid;
  place-items: center;
  border-radius: 10px;
  background: #000;
  box-shadow: 0 0 0 4px #fff;
}
.qr-frame.dimmed .qr-mark { opacity: 0; }
.qr-state {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: #000;
  text-align: center;
  padding: 16px;
}
.state-icon { width: 56px; height: 56px; display: grid; place-items: center; border-radius: 50%; color: #fff; }
.state-icon.blue { background: #007aff; }
.state-icon.green { background: #34c759; }
.state-icon.red { background: #ff3b30; }
.qr-state .btn-bordered { color: #007aff; background: rgba(118, 118, 128, 0.12); }
.steps { list-style: none; display: grid; gap: 12px; max-width: 320px; }
.steps li { display: flex; gap: 12px; align-items: flex-start; color: var(--label-2); }
.steps span {
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  display: grid;
  place-items: center;
  border-radius: 50%;
  font-size: 13px;
  font-weight: 600;
  color: var(--label);
  background: var(--fill-3);
}
.center { text-align: center; max-width: 320px; }
</style>
