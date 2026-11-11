// fetch for the auth API: JSON in and out, the server's `detail` as the error message, and
// back to /login (remembering this page) when the session is gone.
export async function api(path, { method = 'GET', body, signedOut = 'redirect' } = {}) {
  const res = await fetch(path, {
    method,
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  if (res.status === 401 && signedOut === 'redirect') {
    signIn()
    throw new Error('Tu sesión ha caducado')
  }
  const data = res.headers.get('content-type')?.includes('json') ? await res.json() : null
  if (!res.ok) {
    const detail = data?.detail
    const err = new Error(typeof detail === 'string' ? detail : 'Algo ha fallado. Vuelve a intentarlo.')
    err.status = res.status
    throw err
  }
  return data
}

export function signIn() {
  const here = window.location.pathname + window.location.search
  window.location.href = `/auth/signin?next=${encodeURIComponent(here)}`
}

export async function signOut() {
  await fetch('/auth/passkey/logout', { method: 'POST' })
  window.location.href = '/login'
}
