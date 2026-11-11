// WebAuthn's JSON helpers, with a fallback for browsers that don't have them yet (Safari before
// 18.4): the server speaks the JSON form, the browser API wants ArrayBuffers.

const fromB64url = s => Uint8Array.from(atob(s.replace(/-/g, '+').replace(/_/g, '/') + '==='.slice((s.length + 3) % 4)), c => c.charCodeAt(0)).buffer
const toB64url = buf => btoa(String.fromCharCode(...new Uint8Array(buf))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')

export const passkeysSupported = () => typeof window.PublicKeyCredential === 'function'

export async function conditionalSupported() {
  return passkeysSupported() && (await PublicKeyCredential.isConditionalMediationAvailable?.()) === true
}

function requestOptions(json) {
  if (PublicKeyCredential.parseRequestOptionsFromJSON) return PublicKeyCredential.parseRequestOptionsFromJSON(json)
  return {
    ...json,
    challenge: fromB64url(json.challenge),
    allowCredentials: (json.allowCredentials || []).map(c => ({ ...c, id: fromB64url(c.id) })),
  }
}

function creationOptions(json) {
  if (PublicKeyCredential.parseCreationOptionsFromJSON) return PublicKeyCredential.parseCreationOptionsFromJSON(json)
  return {
    ...json,
    challenge: fromB64url(json.challenge),
    user: { ...json.user, id: fromB64url(json.user.id) },
    excludeCredentials: (json.excludeCredentials || []).map(c => ({ ...c, id: fromB64url(c.id) })),
  }
}

function credentialJSON(cred) {
  try { return cred.toJSON() } catch { /* fall through */ }
  const r = cred.response
  const response = { clientDataJSON: toB64url(r.clientDataJSON) }
  if (r.attestationObject) {
    response.attestationObject = toB64url(r.attestationObject)
    response.transports = r.getTransports?.() || []
  } else {
    response.authenticatorData = toB64url(r.authenticatorData)
    response.signature = toB64url(r.signature)
    response.userHandle = r.userHandle ? toB64url(r.userHandle) : null
  }
  return {
    id: cred.id,
    rawId: toB64url(cred.rawId),
    type: cred.type,
    response,
    clientExtensionResults: cred.getClientExtensionResults?.() || {},
    authenticatorAttachment: cred.authenticatorAttachment,
  }
}

export async function getCredential(optionsJSON, { mediation, signal } = {}) {
  const cred = await navigator.credentials.get({ publicKey: requestOptions(optionsJSON), mediation, signal })
  return credentialJSON(cred)
}

export async function createCredential(optionsJSON) {
  const cred = await navigator.credentials.create({ publicKey: creationOptions(optionsJSON) })
  return credentialJSON(cred)
}
