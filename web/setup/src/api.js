// The setup API of a host that isn't set up yet (cli/src/setup_server.rs). The pairing token is
// kept for this tab only.
const KEY = 'tpcloud-setup-token'

export function token() {
  try { return sessionStorage.getItem(KEY) || '' } catch { return '' }
}

function remember(value) {
  try { sessionStorage.setItem(KEY, value) } catch { /* private mode: lives in memory only */ }
  memory = value
}
let memory = ''

export class ApiError extends Error {
  constructor(message, status, body) {
    super(message)
    this.status = status
    this.body = body
  }
}

async function call(method, path, body) {
  const res = await fetch(path, {
    method,
    headers: {
      ...(body ? { 'Content-Type': 'application/json' } : {}),
      ...((token() || memory) ? { Authorization: `Bearer ${token() || memory}` } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  })
  const data = await res.json().catch(() => ({}))
  if (!res.ok) throw new ApiError(data.detail || 'Something went wrong. Try again.', res.status, data)
  return data
}

export const hello = () => call('GET', '/api/setup')
export async function claim(code) {
  const { token: t } = await call('POST', '/api/setup/claim', { code })
  remember(t)
}
export const facts = () => call('GET', '/api/setup/facts')
export const check = answers => call('POST', '/api/setup/check', answers)
export const apply = answers => call('POST', '/api/setup/apply', answers)
export const progress = () => call('GET', '/api/setup/progress')
