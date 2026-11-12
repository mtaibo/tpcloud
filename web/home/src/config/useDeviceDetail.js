import { ref, reactive, computed, nextTick } from 'vue'
import { api } from '@/config/api'
import { useDevices } from '@/config/devices'
import { ask, toast } from '@/ui/dialog.js'
import { t } from '@/ui/i18n.js'

// Asks before something that can't be undone; resolves to true to go ahead.
const confirmAction = (title, message, label, destructive = false) => ask({
    title,
    message,
    actions: [
        { label: t('Cancel', 'Cancelar'), role: 'cancel', value: false },
        { label, role: destructive ? 'destructive' : 'default', value: true },
    ],
})

// The API's metadata names each preference in Spanish.
const LABELS_EN = {
    'Tiempo de subida': 'Raise time', 'Tiempo de bajada': 'Lower time', 'Posición de bajada': 'Lowered position',
    'Invertir relés': 'Swap relays', Ancho: 'Width', Alto: 'Height',
}
const label = es => t(LABELS_EN[es] ?? es, es)

export function useDeviceDetail() {

    const store = useDevices()

    const selectedDevice   = ref(null)
    const devicePrefs      = reactive({})
    const originalPrefs    = reactive({})
    const deviceMap        = reactive({})
    const originalMap      = reactive({})
    const firmwareList     = ref([])
    const selectedFirmware = ref(null)
    const savingPrefs      = ref(false)
    const savingMap        = ref(false)
    const flashingFirmware = ref(false)
    const focusedPrefKey   = ref(null)

    const prefLabels  = reactive({})
    const mapLabels   = reactive({})
    const timeKeys    = ref(new Set())
    const percentKeys = ref(new Set())

    const prefsChanged = computed(() =>
        Object.keys(devicePrefs).some(k => String(devicePrefs[k]) !== String(originalPrefs[k]))
    )
    const mapChanged = computed(() =>
        Object.keys(deviceMap).some(k => String(deviceMap[k]) !== String(originalMap[k]))
    )

    function formatPrefDisplay(key, value) {
        if (timeKeys.value.has(key))    return (Number(value) / 100).toFixed(2) + ' s'
        if (percentKeys.value.has(key)) return (Number(value) / 100).toFixed(2) + ' %'
        return value
    }

    async function startEditPref(key) {
        focusedPrefKey.value = key
        await nextTick()
        document.querySelector(`[data-pref-input="${key}"]`)?.focus()
    }

    function resetObj(obj, source) {
        Object.keys(obj).forEach(k => delete obj[k])
        Object.assign(obj, source)
    }

    async function open(device) {
        selectedDevice.value = device
        resetObj(devicePrefs,   device.prefs ?? {})
        resetObj(originalPrefs, device.prefs ?? {})
        resetObj(deviceMap,     device.map   ?? {})
        resetObj(originalMap,   device.map   ?? {})
        selectedFirmware.value = null
        firmwareList.value = []

        try { firmwareList.value = await api.getFirmwares() } catch {}

        try {
            const meta      = await api.getMetadata()
            const category  = device.category ?? 'blinds'
            const catMeta   = meta[category] ?? {}

            resetObj(prefLabels, Object.fromEntries(
                Object.entries(catMeta.prefs ?? {}).map(([k, v]) => [k, label(v.label)])
            ))
            resetObj(mapLabels, Object.fromEntries(
                Object.entries(catMeta.map ?? {}).map(([k, v]) => [k, label(v.label)])
            ))
            timeKeys.value    = new Set(Object.entries(catMeta.prefs ?? {}).filter(([, v]) => v.type === 'time').map(([k]) => k))
            percentKeys.value = new Set(Object.entries(catMeta.prefs ?? {}).filter(([, v]) => v.type === 'percent').map(([k]) => k))
        } catch {}
    }

    function close() { selectedDevice.value = null }

    async function ping() {
        try { await api.sendCommand(selectedDevice.value.id, 'ping'); toast(t('Ping sent', 'Ping enviado')) }
        catch (e) { console.error('TPHome - Ping error:', e); toast(t('Couldn’t send the ping', 'No se pudo enviar el ping'), 'warning') }
    }

    async function getInfo() {
        try { await api.getDeviceInfo(selectedDevice.value.id); toast(t('Information requested', 'Información pedida')) }
        catch (e) { console.error('TPHome - Info error:', e); toast('No se pudo pedir la información', 'warning') }
    }

    async function toggleLock() {
        const { id, name, category } = selectedDevice.value
        const locked = !store.storage[category]?.[id]?.locked
        const ok = locked
            ? await confirmAction(t(`Lock ${name}?`, `¿Bloquear ${name}?`), t('Its lights will go off and it won’t move, from its buttons or from here, until an administrator unlocks it.', 'Se apagarán sus luces y no se podrá mover ni con los botones ni desde aquí hasta que un administrador la desbloquee.'), t('Lock', 'Bloquear'), true)
            : await confirmAction(t(`Unlock ${name}?`, `¿Desbloquear ${name}?`), t('It can be moved again with its buttons and from here.', 'Se podrá volver a mover con sus botones y desde aquí.'), t('Unlock', 'Desbloquear'))
        if (!ok) return
        try {
            await api.setLocked(id, locked)
            store.storage[category][id].locked = locked
            toast(locked ? t('Locked', 'Bloqueada') : t('Unlocked', 'Desbloqueada'))
        } catch (e) {
            const detail = e.response?.data?.detail
            toast(detail ? t(`Couldn’t ${locked ? 'lock' : 'unlock'}: ${detail}`, `No se pudo ${locked ? 'bloquear' : 'desbloquear'}: ${detail}`) : t('Couldn’t change the lock', 'No se pudo cambiar el bloqueo'), 'warning')
            console.error('TPHome - Lock error:', e)
        }
    }

    async function resetPosition() {
        if (!await confirmAction(t(`Reset the position of ${selectedDevice.value.name}?`, `¿Reiniciar la posición de ${selectedDevice.value.name}?`), t('The switch will assume the blind is at 50%, without moving it.', 'El interruptor dará por hecho que la persiana está al 50 %, sin moverla.'), t('Reset', 'Reiniciar'))) return
        try { await api.resetPosition(selectedDevice.value.id); toast(t('Position reset to 50%', 'Posición reiniciada al 50 %')) }
        catch (e) { console.error('TPHome - Reset position error:', e) }
    }

    async function deleteDevice() {
        if (!await confirmAction(t(`Delete ${selectedDevice.value.name}?`, `¿Eliminar ${selectedDevice.value.name}?`), t('It will disappear from the house and the map. The switch will have to be set up again.', 'Desaparecerá de la casa y del plano. El interruptor tendrá que volver a configurarse.'), t('Delete', 'Eliminar'), true)) return
        try {
            await api.deleteDevice(selectedDevice.value.id)
            close()
            await store.setup()
        } catch (e) { console.error('TPHome - Delete error:', e) }
    }

    async function savePrefs() {
        savingPrefs.value = true
        try {
            const { id, category } = selectedDevice.value
            const original = selectedDevice.value.prefs ?? {}
            const parsed = {}
            for (const [k, v] of Object.entries(devicePrefs)) {
                const orig = original[k]
                if (typeof orig === 'number')       parsed[k] = Number(v)
                else if (typeof orig === 'boolean') parsed[k] = (v === 'true' || v === true)
                else parsed[k] = v
            }
            await api.patchDeviceConfig(id, 'prefs', parsed)
            store.storage[category][id].prefs = parsed
            await api.sendPrefs(id, parsed)
            resetObj(originalPrefs, parsed)
            toast(t('Preferences saved and sent', 'Preferencias guardadas y enviadas'))
        } catch (e) { console.error('TPHome - Save prefs error:', e); toast('No se pudieron guardar', 'warning') }
        finally { savingPrefs.value = false }
    }

    async function saveMap() {
        savingMap.value = true
        try {
            const { id, category } = selectedDevice.value
            const parsed = {}
            for (const [k, v] of Object.entries(deviceMap)) parsed[k] = Number(v)
            await api.patchDeviceConfig(id, 'map', parsed)
            store.storage[category][id].map = parsed
            resetObj(originalMap, parsed)
            toast(t('Map position saved', 'Posición en el plano guardada'))
        } catch (e) { console.error('TPHome - Save map error:', e); toast('No se pudo guardar', 'warning') }
        finally { savingMap.value = false }
    }

    async function flashFirmware() {
        if (!selectedFirmware.value) return
        if (!await confirmAction(t(`Install version ${selectedFirmware.value} on ${selectedDevice.value.name}?`, `¿Instalar la versión ${selectedFirmware.value} en ${selectedDevice.value.name}?`), t('The switch will download it and restart.', 'El interruptor la descargará y se reiniciará.'), t('Install', 'Instalar'))) return
        flashingFirmware.value = true
        try { await api.sendOTA(selectedDevice.value.id, selectedFirmware.value); toast(t('Update sent', 'Actualización enviada')) }
        catch (e) { console.error('TPHome - OTA error:', e); toast('No se pudo enviar la actualización', 'warning') }
        finally { flashingFirmware.value = false }
    }

    return {
        selectedDevice, devicePrefs, deviceMap,
        firmwareList, selectedFirmware,
        savingPrefs, savingMap, flashingFirmware, focusedPrefKey,
        prefLabels, mapLabels, timeKeys, percentKeys,
        prefsChanged, mapChanged,
        formatPrefDisplay, startEditPref,
        open, close, ping, getInfo, toggleLock, resetPosition, deleteDevice,
        savePrefs, saveMap, flashFirmware,
    }
}
