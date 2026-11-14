<script setup>
import { computed, onMounted, ref } from 'vue'
import { Check, HardDriveDownload, MoreHorizontal, Trash2 } from 'lucide-vue-next'

import ListSection from '@/ui/ListSection.vue'
import ListRow     from '@/ui/ListRow.vue'
import Segmented   from '@/ui/Segmented.vue'
import Spinner     from '@/ui/Spinner.vue'

import { api } from '@/config/api'
import { ask, toast } from '@/ui/dialog.js'
import { openMenu } from '@/menu.js'
import { locale, t as tr } from '@/ui/i18n.js'

// The binaries the switches update from: upload a new one, mark which is active, delete old ones.
const firmwares = ref([])
const uploading = ref(false)
const fileInput = ref(null)
const selectedFile = ref(null)
const form = ref({ name: '', chip: '', target: 'blinds', version: '', notes: '' })

const targets = computed(() => [
    { value: 'blinds', label: tr('Blinds', 'Persianas') },
    { value: 'lights', label: tr('Lights', 'Luces') },
    { value: 'switches', label: tr('Switches', 'Interruptores') },
])
const targetName = t => targets.value.find(x => x.value === t)?.label ?? t
const canUpload = computed(() => selectedFile.value && form.value.name && form.value.chip && form.value.version && !uploading.value)

onMounted(load)

async function load() {
    try { firmwares.value = await api.getFirmwares() }
    catch (e) { console.error('TPHome - Error loading firmwares:', e) }
}

function onFile(e) {
    const file = e.target.files[0]
    selectedFile.value = file && file.name.endsWith('.bin') ? file : null
    if (file && !selectedFile.value) toast(tr('It has to be a .bin file', 'Tiene que ser un archivo .bin'), 'warning')
    e.target.value = ''
}

async function upload() {
    if (!canUpload.value) return
    uploading.value = true
    try {
        await api.uploadFirmware(selectedFile.value, { ...form.value })
        form.value = { name: '', chip: '', target: 'blinds', version: '', notes: '' }
        selectedFile.value = null
        toast(tr('Firmware uploaded', 'Firmware subido'))
        await load()
    } catch (e) {
        console.error('TPHome - Upload error:', e)
        toast(tr('Couldn’t upload', 'No se pudo subir'), 'warning')
    } finally {
        uploading.value = false
    }
}

async function activate(fw) {
    try { await api.activateFirmware(fw.id); toast(tr(`v${fw.version} is now the active one`, `v${fw.version} es ahora la activa`)); await load() }
    catch (e) { console.error('TPHome - Activate error:', e); toast(tr('Couldn’t activate', 'No se pudo activar'), 'warning') }
}

async function remove(fw) {
    const ok = await ask({
        title: tr(`Delete ${fw.name} v${fw.version}?`, `¿Eliminar ${fw.name} v${fw.version}?`),
        message: tr('The switches won’t be able to install this version any more.', 'Los interruptores ya no podrán instalar esta versión.'),
        actions: [
            { label: tr('Cancel', 'Cancelar'), role: 'cancel', value: false },
            { label: tr('Delete', 'Eliminar'), role: 'destructive', value: true },
        ],
    })
    if (!ok) return
    try { await api.deleteFirmware(fw.id); await load() }
    catch (e) { console.error('TPHome - Delete error:', e); toast(tr('Couldn’t delete', 'No se pudo eliminar'), 'warning') }
}

function menu(e, fw) {
    openMenu(e.currentTarget, [
        !fw.active && { label: tr('Mark as active', 'Marcar como activa'), icon: Check, action: () => activate(fw) },
        { label: tr('Delete', 'Eliminar'), icon: Trash2, danger: true, action: () => remove(fw) },
    ])
}

const when = d => new Date(d).toLocaleDateString(locale(), { day: 'numeric', month: 'short', year: 'numeric' })
</script>

<template>
    <div class="view">
        <div class="page">
            <header class="page-header">
                <h1 class="large-title">Firmware</h1>
            </header>

            <div class="columns">
                <section>
                    <ListSection :header="tr('Upload firmware', 'Subir firmware')">
                        <ListRow :title="tr('File', 'Archivo')" clickable @click="fileInput.click()">
                            <template #trailing>
                                <span class="row-value file">{{ selectedFile ? selectedFile.name : tr('Choose .bin…', 'Elegir .bin…') }}</span>
                            </template>
                        </ListRow>
                        <ListRow :title="tr('Name', 'Nombre')">
                            <template #trailing><input v-model="form.name" class="row-input trailing" :placeholder="tr('Blind', 'Persiana')" /></template>
                        </ListRow>
                        <ListRow :title="tr('Version', 'Versión')">
                            <template #trailing><input v-model="form.version" class="row-input trailing" placeholder="2.2.0" /></template>
                        </ListRow>
                        <ListRow title="Chip">
                            <template #trailing><input v-model="form.chip" class="row-input trailing" placeholder="ESP8266" /></template>
                        </ListRow>
                        <div class="row target">
                            <Segmented v-model="form.target" :options="targets" />
                        </div>
                        <div class="row notes">
                            <textarea v-model="form.notes" class="row-input" rows="2" :placeholder="tr('Notes or changes', 'Notas o cambios')" />
                        </div>
                    </ListSection>
                    <input ref="fileInput" type="file" accept=".bin" hidden @change="onFile" />
                    <button class="btn btn-prominent block upload" type="button" :disabled="!canUpload" @click="upload">
                        <Spinner v-if="uploading" :size="18" style="color: #fff" />
                        <span v-else>{{ tr('Upload', 'Subir') }}</span>
                    </button>
                </section>

                <section>
                    <ListSection v-if="firmwares.length" :header="tr('Versions', 'Versiones')" :footer="tr('The active one is offered when installing from a device.', 'La activa es la que se ofrece al instalar desde un dispositivo.')">
                        <ListRow
                            v-for="fw in firmwares"
                            :key="fw.id"
                            :title="`${fw.name} v${fw.version}`"
                            :subtitle="[fw.chip, targetName(fw.target), when(fw.uploaded_at), fw.notes].filter(Boolean).join(' · ')"
                            small-subtitle
                        >
                            <template #leading>
                                <span class="row-icon" :style="{ background: fw.active ? 'var(--green)' : 'var(--gray)' }"><HardDriveDownload :size="17" :stroke-width="2" /></span>
                            </template>
                            <template #trailing>
                                <span v-if="fw.active" class="badge green">{{ tr('Active', 'Activa') }}</span>
                                <button class="more" type="button" :aria-label="tr('More', 'Más')" @click="menu($event, fw)"><MoreHorizontal :size="18" :stroke-width="2.2" /></button>
                            </template>
                        </ListRow>
                    </ListSection>
                    <div v-else class="empty">
                        <HardDriveDownload :size="44" :stroke-width="1.5" class="empty-icon" />
                        <h3>{{ tr('No firmware', 'Sin firmwares') }}</h3>
                        <p class="subheadline">{{ tr('Upload a .bin to be able to update the switches.', 'Sube un .bin para poder actualizar los interruptores.') }}</p>
                    </div>
                </section>
            </div>
        </div>
    </div>
</template>

<style scoped>
.columns { display: grid; gap: 0 24px; }
@media (min-width: 1100px) { .columns { grid-template-columns: 1fr 1fr; align-items: start; } }
.file { max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--accent); }
.row-input.trailing { width: 150px; }
.target { padding-top: 10px; padding-bottom: 10px; }
.target .segmented { width: 100%; }
.notes textarea { resize: none; font-size: 17px; padding: 6px 0; }
.upload { margin: -12px 0 26px; }
.more { width: 32px; height: 32px; display: grid; place-items: center; border-radius: 50%; color: var(--accent); background: var(--fill-3); }
.more:hover { background: var(--fill-2); }
</style>
