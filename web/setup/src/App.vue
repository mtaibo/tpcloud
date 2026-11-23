<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import ListSection from '@ui/ListSection.vue'
import ListRow from '@ui/ListRow.vue'
import Toggle from '@ui/Toggle.vue'
import Segmented from '@ui/Segmented.vue'
import Spinner from '@ui/Spinner.vue'
import Logo from '@ui/Logo.vue'
import Icon from '@ui/Icon.vue'
import * as api from './api.js'

// The same questions as `sudo tpcloud setup` in a terminal, asked one screen at a time, and
// applied by the same code on the host (cli/src/engine.rs).

const hello = ref(null)
const facts = ref(null)
const answers = ref(null)
const step = ref('loading')
const error = ref('')
const problems = ref([])
const busy = ref(false)
const code = ref('')
const password2 = ref('')
const erase = ref('')
const run = ref(null)
let timer = null

const steps = computed(() => {
  const list = ['name', 'files', 'backups', 'modules', 'home', 'outside']
  if (facts.value?.needs_owner) list.push('account')
  list.push('review')
  return list
})
const index = computed(() => steps.value.indexOf(step.value))

const titles = {
  pair: 'Set up TPCloud',
  name: 'Name it',
  files: 'Files',
  backups: 'Backups',
  modules: 'What it does',
  home: 'At home',
  outside: 'From outside',
  account: 'Your account',
  review: 'Ready',
  apply: 'Setting it up',
  done: 'All set',
}

const freeDisks = computed(() => (facts.value?.disks || []).filter(d => !d.busy))
const chosenDisks = computed(() => (facts.value?.disks || []).filter(d => answers.value?.files.disks?.includes(d.path)))
const levels = computed(() => {
  const n = chosenDisks.value.length
  if (n <= 1) return [{ value: 'single', label: 'One disk' }]
  const list = [{ value: 'raid1', label: 'Mirror' }]
  if (n >= 4) list.push({ value: 'raid10', label: 'Mirrored pairs' })
  list.push({ value: 'single', label: 'All the space' })
  return list
})
const levelText = {
  single: 'Every file once, all the space. A disk that fails takes its files with it.',
  raid1: 'Every file on two disks: one can fail and nothing is lost. About half the space.',
  raid10: 'Pairs of mirrored disks, faster: one per pair can fail.',
}
watch(levels, list => {
  if (answers.value?.files.kind === 'pool' && !list.some(l => l.value === answers.value.files.level)) answers.value.files.level = list[0].value
})

const filesMode = computed({
  get: () => answers.value?.files.kind || 'folder',
  set: kind => {
    if (kind === 'pool') answers.value.files = { kind: 'pool', disks: freeDisks.value.length === 1 ? [freeDisks.value[0].path] : [], level: 'raid1', encrypt: true }
    else answers.value.files = { kind: 'folder', path: folderBefore.value }
  },
})
const folderBefore = ref('/var/lib/tpcloud/files')
const backupsOn = computed({
  get: () => backupsWanted.value,
  set: on => { backupsWanted.value = on; if (!on) answers.value.backup_dir = '' },
})
const backupsWanted = ref(false)
const otherName = computed({
  get: () => (answers.value?.local_names || []).join(', '),
  set: v => { answers.value.local_names = v.split(',').map(s => s.trim()).filter(Boolean) },
})
const domainKind = computed({
  get: () => answers.value?.domain.kind || 'none',
  set: kind => { answers.value.domain = { kind, hostname: answers.value.domain.hostname || '', new: kind === 'cloudflare' ? (answers.value.domain.new ?? true) : undefined } },
})

function problemsFor(field) {
  return problems.value.filter(p => p.field === field || p.field.startsWith(field)).map(p => p.message)
}

function toggleDisk(path) {
  const list = answers.value.files.disks
  const i = list.indexOf(path)
  if (i >= 0) list.splice(i, 1)
  else list.push(path)
  erase.value = ''
}

// What each screen must have before going on (the host checks everything again before applying).
const stepProblem = computed(() => {
  const a = answers.value
  if (!a) return ''
  switch (step.value) {
    case 'name': return a.name.trim() ? '' : 'Give it a name.'
    case 'files':
      if (a.files.kind === 'folder') return a.files.path?.startsWith('/') ? '' : 'Give the full path of a folder, starting with /.'
      if (!a.files.disks.length) return 'Pick at least one disk.'
      return erase.value === 'ERASE' ? '' : 'Type ERASE to confirm the disks can be erased.'
    case 'backups': return backupsWanted.value && !a.backup_dir.startsWith('/') ? 'Give the full path of the backups folder.' : ''
    case 'outside':
      if (a.domain.kind !== 'none' && !a.domain.hostname.includes('.')) return 'Give the full address, like cloud.example.com.'
      if (!a.local && !a.tailscale && a.domain.kind === 'none') return 'Then nothing could reach it: pick at least one way.'
      return ''
    case 'account':
      if (!a.owner.email.includes('@')) return 'Enter your email.'
      if (a.owner.password.length < 12) return 'The password needs 12 characters or more.'
      return a.owner.password === password2.value ? '' : "The passwords don't match."
    default: return ''
  }
})

async function start() {
  try {
    hello.value = await api.hello()
    if (api.token()) {
      try { return await load() } catch (e) { if (e.status !== 401) throw e }
    }
    step.value = 'pair'
  } catch (e) {
    error.value = e.message
    step.value = 'pair'
  }
}

async function load() {
  const data = await api.facts()
  facts.value = data.facts
  const a = data.answers
  if (data.facts.needs_owner) a.owner = { email: '', password: '' }
  if (a.files.kind === 'folder') folderBefore.value = a.files.path
  backupsWanted.value = !!a.backup_dir
  answers.value = a
  const p = await api.progress()
  if (p.state !== 'idle') { run.value = p; step.value = p.state === 'done' ? 'done' : 'apply'; poll() }
  else step.value = 'name'
}

async function pair() {
  busy.value = true
  error.value = ''
  try {
    await api.claim(code.value.trim())
    await load()
  } catch (e) {
    error.value = e.message
  } finally {
    busy.value = false
  }
}

async function next() {
  error.value = ''
  if (stepProblem.value) { error.value = stepProblem.value; return }
  if (step.value === 'review') return applyNow()
  if (step.value === steps.value[steps.value.length - 2]) {
    // Before the summary, the host's own check of everything.
    busy.value = true
    try {
      problems.value = (await api.check(answers.value)).problems
    } catch (e) {
      error.value = e.message
      return
    } finally {
      busy.value = false
    }
    const first = problems.value[0]
    if (first) {
      const where = { name: 'name', files: 'files', backup_dir: 'backups', local_names: 'home', domain: 'outside', reach: 'outside', owner_email: 'account', owner_password: 'account' }[first.field]
      if (where) step.value = where
      error.value = first.message
      return
    }
  }
  step.value = steps.value[index.value + 1]
  window.scrollTo({ top: 0 })
}

function back() {
  error.value = ''
  if (index.value > 0) step.value = steps.value[index.value - 1]
}

async function applyNow(overwrite = false) {
  busy.value = true
  error.value = ''
  try {
    await api.apply({ ...answers.value, overwrite_dns: overwrite })
    step.value = 'apply'
    run.value = { state: 'running', log: [] }
    poll()
  } catch (e) {
    error.value = e.body?.problems?.[0]?.message || e.message
  } finally {
    busy.value = false
  }
}

function poll() {
  clearTimeout(timer)
  timer = setTimeout(async () => {
    try { run.value = await api.progress() } catch { /* the host restarts as TPCloud at the end */ }
    if (run.value?.state === 'done') { step.value = 'done'; return }
    if (run.value?.state !== 'failed') poll()
  }, 1000)
}

const recoverySaved = ref(false)
const mainUrl = computed(() => run.value?.url || '')
const localUrl = computed(() => [mainUrl.value, ...(run.value?.others || [])].find(u => u.endsWith('.local')) || '')

onMounted(start)
onBeforeUnmount(() => clearTimeout(timer))
</script>

<template>
  <main class="setup">
    <div class="column">
      <header class="hero">
        <Logo :size="64" />
        <h1 class="title-1">{{ titles[step] || 'Set up TPCloud' }}</h1>
        <p v-if="hello" class="subheadline secondary">
          <template v-if="step === 'pair'">This machine, <b>{{ hello.host }}</b>, has TPCloud {{ hello.version }} and nothing set up yet.</template>
          <template v-else-if="index >= 0">Step {{ index + 1 }} of {{ steps.length }}</template>
        </p>
      </header>

      <div v-if="step === 'loading'" class="center"><Spinner :size="28" /></div>

      <!-- Pairing -->
      <form v-else-if="step === 'pair'" class="stack" @submit.prevent="pair">
        <ListSection :footer="hello?.open
          ? 'Shown where TPCloud was installed, and by `sudo tpcloud status`. For a few minutes after it starts, a device on this network can also go on without it.'
          : 'Shown where TPCloud was installed, and by `sudo tpcloud status` on this machine.'">
          <ListRow>
            <input v-model="code" class="row-input code" :class="{ filled: code }" inputmode="numeric" maxlength="6" autocomplete="one-time-code" placeholder="Pairing code" aria-label="Pairing code" />
          </ListRow>
        </ListSection>
        <button class="btn btn-prominent block" type="submit" :disabled="busy || (!hello?.open && code.length !== 6)">
          <Spinner v-if="busy" :size="18" style="color:#fff" /><span v-else>{{ code ? 'Continue' : 'Continue without a code' }}</span>
        </button>
      </form>

      <!-- Name -->
      <div v-else-if="step === 'name'" class="stack">
        <ListSection footer="What it's called in the apps: Home, the family's name…">
          <ListRow><input v-model="answers.name" class="row-input" maxlength="64" placeholder="Name" aria-label="Name" @keydown.enter="next" /></ListRow>
        </ListSection>
      </div>

      <!-- Files -->
      <div v-else-if="step === 'files'" class="stack">
        <p v-if="facts.pool" class="footnote secondary pad">Storage pool here: {{ facts.pool }}.</p>
        <Segmented v-if="freeDisks.length && !facts.pool" v-model="filesMode" :options="[{ value: 'folder', label: 'A folder' }, { value: 'pool', label: 'New storage' }]" />
        <ListSection v-if="filesMode === 'folder'" footer="On a disk that's already set up: nothing is erased. A big disk is best.">
          <ListRow><input v-model="answers.files.path" class="row-input mono" spellcheck="false" placeholder="/path/to/folder" aria-label="Folder" /></ListRow>
        </ListSection>
        <template v-else>
          <ListSection header="Disks" footer="TPCloud formats the ones you pick. The system's disk and disks in use can't be picked.">
            <ListRow v-for="d in facts.disks" :key="d.path" :title="`${d.path} · ${d.size_text}`" :subtitle="d.busy || (d.contents.length ? `Has ${d.contents.join(', ')}` : d.model || 'Empty')" :disabled="!!d.busy" :clickable="!d.busy" @click="toggleDisk(d.path)">
              <template #trailing><span v-if="answers.files.disks.includes(d.path)" class="tint"><Icon name="checkmark" :size="20" :weight="2.4" /></span></template>
            </ListRow>
          </ListSection>
          <template v-if="chosenDisks.length">
            <Segmented v-if="levels.length > 1" v-model="answers.files.level" :options="levels" />
            <p class="footnote secondary pad">{{ levelText[answers.files.level] }}</p>
            <ListSection footer="A disk taken away on its own can't be read. This machine opens them by itself; you get a recovery key for anywhere else.">
              <ListRow title="Encrypt"><template #trailing><Toggle v-model="answers.files.encrypt" label="Encrypt" /></template></ListRow>
            </ListSection>
            <ListSection :footer="`Everything on ${chosenDisks.map(d => d.path).join(', ')} is erased. Type ERASE to confirm.`">
              <ListRow><input v-model="erase" class="row-input" autocapitalize="characters" spellcheck="false" placeholder="ERASE" aria-label="Type ERASE" /></ListRow>
            </ListSection>
          </template>
        </template>
      </div>

      <!-- Backups -->
      <div v-else-if="step === 'backups'" class="stack">
        <ListSection footer="Every night, a copy of TPCloud's own data (accounts, the photo index, albums, share links, the house, settings: a few MB), best on another disk. Not your files: back that disk up on its own.">
          <ListRow title="Nightly backups"><template #trailing><Toggle v-model="backupsOn" label="Nightly backups" /></template></ListRow>
          <ListRow v-if="backupsWanted"><input v-model="answers.backup_dir" class="row-input mono" spellcheck="false" placeholder="/path/to/backups" aria-label="Backups folder" /></ListRow>
        </ListSection>
      </div>

      <!-- Modules -->
      <div v-else-if="step === 'modules'" class="stack">
        <ListSection>
          <ListRow title="Store" subtitle="Files, photo backup from the app, share links"><template #trailing><Toggle v-model="answers.store" label="Store" /></template></ListRow>
          <ListRow title="Home" subtitle="Blinds and lights with tphome's switches"><template #trailing><Toggle v-model="answers.home" label="Home" /></template></ListRow>
        </ListSection>
      </div>

      <!-- At home -->
      <div v-else-if="step === 'home'" class="stack">
        <ListSection :footer="`On this network the apps find it by themselves at https://${facts.host}, with this machine's own certificate.`">
          <ListRow title="Answer on this network"><template #trailing><Toggle v-model="answers.local" label="Answer on this network" /></template></ListRow>
        </ListSection>
        <ListSection v-if="answers.local" footer="If your router's DNS gives this machine a name of its own (cloud.casa…). Optional.">
          <ListRow><input v-model="otherName" class="row-input" spellcheck="false" autocapitalize="off" placeholder="Another name" aria-label="Another name" /></ListRow>
        </ListSection>
      </div>

      <!-- From outside -->
      <div v-else-if="step === 'outside'" class="stack">
        <ListSection :footer="facts.tailscale ? `As ${facts.tailscale}: only your tailnet's devices. MagicDNS and HTTPS have to be on.` : 'Install Tailscale on this machine and sign in to use it.'">
          <ListRow title="Tailscale" :disabled="!facts.tailscale"><template #trailing><Toggle v-model="answers.tailscale" label="Tailscale" :disabled="!facts.tailscale" /></template></ListRow>
        </ListSection>
        <p class="section-header pad">Your own domain</p>
        <Segmented v-model="domainKind" :options="[{ value: 'none', label: 'No' }, { value: 'cloudflare', label: 'Cloudflare' }, { value: 'open_port', label: 'Open port' }]" />
        <template v-if="answers.domain.kind !== 'none'">
          <ListSection :footer="answers.domain.kind === 'cloudflare' ? 'A tunnel, no ports opened. You sign in to Cloudflare once, on any device, and pick the domain.' : 'Port 443 forwarded to this machine; the certificate comes by itself.'">
            <ListRow><input v-model="answers.domain.hostname" class="row-input" spellcheck="false" autocapitalize="off" placeholder="cloud.example.com" aria-label="Address" /></ListRow>
          </ListSection>
          <ListSection v-if="answers.domain.kind === 'cloudflare' && facts.has_token" footer="The tunnel made in Cloudflare's dashboard keeps its own settings there.">
            <ListRow title="Keep the tunnel that's here"><template #trailing><Toggle :model-value="!answers.domain.new" label="Keep the tunnel" @update:model-value="v => (answers.domain.new = !v)" /></template></ListRow>
          </ListSection>
        </template>
      </div>

      <!-- Account -->
      <form v-else-if="step === 'account'" class="stack" @submit.prevent="next">
        <ListSection footer="The first account, and the admin. At least 12 characters: a phrase of a few words is easy to remember and hard to guess.">
          <ListRow><input v-model="answers.owner.email" class="row-input" type="email" autocomplete="username" placeholder="Email" aria-label="Email" /></ListRow>
          <ListRow><input v-model="answers.owner.password" class="row-input" type="password" autocomplete="new-password" placeholder="Password" aria-label="Password" /></ListRow>
          <ListRow><input v-model="password2" class="row-input" type="password" autocomplete="new-password" placeholder="Same password again" aria-label="Same password again" /></ListRow>
        </ListSection>
        <button type="submit" hidden />
      </form>

      <!-- Review -->
      <div v-else-if="step === 'review'" class="stack">
        <ListSection>
          <ListRow title="Name" :value="answers.name" />
          <ListRow title="Files" :value="answers.files.kind === 'folder' ? answers.files.path : `${chosenDisks.length} disk(s), ${levels.find(l => l.value === answers.files.level)?.label}${answers.files.encrypt ? ', encrypted' : ''}`" wrap-value />
          <ListRow title="Backups" :value="answers.backup_dir || 'Off'" wrap-value />
          <ListRow title="Modules" :value="[answers.store && 'Store', answers.home && 'Home'].filter(Boolean).join(', ') || 'None'" />
          <ListRow title="At home" :value="answers.local ? [facts.host, ...answers.local_names].join(', ') : 'Off'" wrap-value />
          <ListRow title="Tailscale" :value="answers.tailscale ? facts.tailscale : 'Off'" wrap-value />
          <ListRow title="Domain" :value="answers.domain.kind === 'none' ? 'None' : `${answers.domain.hostname} (${answers.domain.kind === 'cloudflare' ? 'Cloudflare' : 'open port'})`" wrap-value />
          <ListRow v-if="answers.owner" title="Account" :value="answers.owner.email" wrap-value />
        </ListSection>
      </div>

      <!-- Applying -->
      <div v-else-if="step === 'apply' || step === 'done'" class="stack">
        <ListSection v-if="run?.log?.length">
          <ListRow v-for="(l, i) in run.log" :key="i" :title="l.text">
            <template #trailing>
              <span v-if="l.kind === 'done' || run.state === 'done' || i < run.log.length - 1" class="green"><Icon name="checkmark" :size="18" :weight="2.4" /></span>
              <Spinner v-else-if="i === run.log.length - 1 && run.state === 'running'" :size="16" />
            </template>
          </ListRow>
        </ListSection>
        <div v-if="run?.state === 'running' && !run.log?.length" class="center"><Spinner :size="28" /></div>

        <ListSection v-if="run?.sign_in && run.state === 'running'" header="Cloudflare" footer="Sign in and pick the domain of the address you gave. This page carries on by itself afterwards.">
          <ListRow title="Open Cloudflare's sign-in" :href="run.sign_in" accent chevron />
        </ListSection>

        <div v-if="run?.recovery_key" class="key-card">
          <p class="headline">Recovery key</p>
          <p class="footnote secondary">This machine opens its disks by itself. If it ever breaks, this key opens them on any Linux. It's shown only now: write it down and keep it safe.</p>
          <p class="key mono">{{ run.recovery_key }}</p>
          <label class="saved footnote"><input v-model="recoverySaved" type="checkbox" /> I've written it down</label>
        </div>

        <template v-if="run?.state === 'failed'">
          <ListSection v-if="run.dns_taken" :footer="`${run.dns_taken} already points somewhere else in Cloudflare.`">
            <ListRow :title="`Point ${run.dns_taken} at this machine`" clickable accent @click="applyNow(true)" />
          </ListSection>
          <p class="error footnote" role="alert"><Icon name="warning" :size="16" :weight="2" /> {{ run.error }}</p>
          <button class="btn btn-glass block" type="button" @click="step = 'review'; run = null">Back to the answers</button>
        </template>

        <template v-if="step === 'done' && (!run?.recovery_key || recoverySaved)">
          <ListSection footer="It restarts as TPCloud in a few seconds. In the apps, add it from the list of networks; in a browser, open its address.">
            <ListRow title="Address" :value="mainUrl" wrap-value />
            <ListRow v-for="o in run.others" :key="o" title="Also" :value="o" wrap-value />
          </ListSection>
          <p v-if="localUrl" class="footnote secondary pad">A browser at home warns about {{ localUrl }} until it trusts this machine's certificate: download it at <a :href="`${localUrl}/auth/network/ca.crt`">{{ localUrl }}/auth/network/ca.crt</a> and add it to your system's certificates. The apps don't need it.</p>
          <a class="btn btn-prominent block" :href="mainUrl">Open TPCloud</a>
        </template>
      </div>

      <p v-if="error && step !== 'apply'" class="error footnote" role="alert"><Icon name="warning" :size="16" :weight="2" /> {{ error }}</p>

      <nav v-if="index >= 0" class="actions">
        <button v-if="index > 0" class="btn btn-glass" type="button" :disabled="busy" @click="back">Back</button>
        <button class="btn btn-prominent grow" type="button" :disabled="busy" @click="next">
          <Spinner v-if="busy" :size="18" style="color:#fff" />
          <span v-else>{{ step === 'review' ? (answers.files.kind === 'pool' ? 'Erase the disks and set it up' : 'Set it up') : 'Continue' }}</span>
        </button>
      </nav>
    </div>
    <footer class="legal footnote tertiary">TPCloud · nothing leaves this network until you choose so</footer>
  </main>
</template>

<style scoped>
.setup { min-height: 100dvh; display: flex; flex-direction: column; align-items: center; padding: 24px 16px calc(24px + env(safe-area-inset-bottom)); }
.column { width: 100%; max-width: 480px; flex: 1; }
.hero { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 24px 0 20px; }
.stack { display: flex; flex-direction: column; gap: 4px; }
.stack > :deep(.segmented) { margin: 0 0 14px; }
.center { display: flex; justify-content: center; padding: 32px 0; }
.pad { padding: 0 16px 12px; }
.code { text-align: center; }
.code.filled { font-size: 22px; letter-spacing: 6px; font-variant-numeric: tabular-nums; }
.actions { display: flex; gap: 10px; margin-top: 12px; }
.actions .grow { flex: 1; }
.error { display: flex; align-items: center; gap: 6px; color: var(--red); padding: 8px 16px; }
.green { color: var(--green); }
.key-card { background: var(--bg-2, var(--fill-3)); border-radius: 14px; padding: 16px; margin: 8px 0 22px; display: flex; flex-direction: column; gap: 8px; }
.key { font-size: 17px; letter-spacing: 1px; word-break: break-all; user-select: all; padding: 10px 0; }
.saved { display: flex; align-items: center; gap: 8px; }
.legal { text-align: center; padding-top: 24px; }
</style>
