import { reactive } from 'vue'
import { syncLang } from '@/ui/i18n.js'

// Who's signed in, from tpcloud's auth (the gateway already let them through).
export const session = reactive({ user: null })

export async function loadUser() {
    const res = await fetch('/auth/passkey/me')
    if (!res.ok) throw new Error('signed out')
    session.user = await res.json()
    syncLang(session.user)
}

export async function signOut() {
    await fetch('/auth/passkey/logout', { method: 'POST' })
    window.location.href = '/login'
}
