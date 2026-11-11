import { reactive, ref } from 'vue'
import { api } from '@ui/api.js'

// What every screen shares: who's signed in and their profile (passkeys, sessions…), reloaded
// after anything changes it.
export const store = reactive({
  me: null,
  profile: null,
  async load() {
    this.me = await api('/auth/passkey/me')
    await this.reload()
  },
  async reload() {
    this.profile = await api('/auth/account/profile')
  },
})

const query = window.matchMedia('(min-width: 900px)')
export const wide = ref(query.matches)
query.addEventListener('change', e => { wide.value = e.matches })
