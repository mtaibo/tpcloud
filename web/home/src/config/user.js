import { reactive } from 'vue'

// Who's signed in, from tpcloud's auth (the gateway already let them through).
export const session = reactive({ user: null })

export async function loadUser() {
    const res = await fetch('/auth/passkey/me')
    if (!res.ok) throw new Error('signed out')
    session.user = await res.json()
}

export async function signOut() {
    await fetch('/auth/passkey/logout', { method: 'POST' })
    window.location.href = '/login'
}
