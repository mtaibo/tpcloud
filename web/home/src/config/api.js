import axios from 'axios'

const client = axios.create({
    baseURL: '/api',
    headers: { 'Content-Type': 'application/json' }
})

export const api = {

    async getDevices() {
        const response = await client.get('/devices')
        return response.data
    },

    async getDevicesByCategory(category) {
        const response = await client.get(`/devices/${category}`)
        return response.data
    },

    async getPending() {
        const response = await client.get('/devices/pending')
        return response.data
    },

    async getConfig(subject) {
        const response = await client.get(`/config/${subject}`)
        return response.data
    },

    async postConfig(subject, newConfig) {
        return client.put(`/config/${subject}`, newConfig)
    },

    async getMetadata() {
        const response = await client.get('/config/metadata')
        return response.data
    },

    async patchDeviceConfig(id, key, data) {
        return client.patch(`/config/devices/${id}/${key}`, data)
    },

    async sendCommand(id, command, value = null) {
        const path = command === 'set' && value !== null
            ? `/commands/${id}/set/${value}`
            : `/commands/${id}/${command}`
        return client.post(path)
    },

    async sendPrefs(id, prefs) {
        return client.post(`/admin/${id}/prefs`, prefs)
    },

    async resetPosition(id) {
        return client.post(`/admin/${id}/set/50`)
    },

    async getDeviceInfo(id) {
        return client.post(`/admin/${id}/info`)
    },

    async configurePending(mac, id, prefs) {
        return client.post('/devices/pending/configure', { mac, id, prefs })
    },

    async deleteDevice(id) {
        return client.delete(`/devices/${id}`)
    },

    async sendOTA(id, version) {
        return client.post(`/admin/${id}/ota`, null, { params: { version } })
    },

    async uploadFirmware(file, metadata) {
        const formData = new FormData()
        formData.append('file', file)
        formData.append('name', metadata.name)
        formData.append('chip', metadata.chip)
        formData.append('target', metadata.target)
        formData.append('version', metadata.version)
        formData.append('notes', metadata.notes || '')
        const response = await client.post('/firmware/upload', formData, {
            headers: { 'Content-Type': 'multipart/form-data' }
        })
        return response.data
    },

    async getFirmwares() {
        const response = await client.get('/firmware/list')
        return response.data
    },

    async activateFirmware(id) {
        return client.post(`/firmware/${id}/activate`)
    },

    async deleteFirmware(id) {
        return client.delete(`/firmware/${id}`)
    },
}
