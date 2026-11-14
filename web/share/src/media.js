import { IMAGE_EXTS, VIDEO_EXTS } from './fileTypes.js'

const MIME_TYPES = {
  jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png', gif: 'image/gif', webp: 'image/webp',
  heic: 'image/heic', heif: 'image/heif', tiff: 'image/tiff', bmp: 'image/bmp',
  mov: 'video/quicktime', mp4: 'video/mp4', m4v: 'video/x-m4v', webm: 'video/webm',
}

function extension(name) {
  return name.split('.').pop()?.toLowerCase() ?? ''
}

export function isImage(entry) {
  return entry.type === 'file' && IMAGE_EXTS.has(extension(entry.name))
}

export function isVideo(entry) {
  return entry.type === 'file' && VIDEO_EXTS.has(extension(entry.name))
}

export function isMedia(entry) {
  return isImage(entry) || isVideo(entry)
}

export function mimeType(name) {
  return MIME_TYPES[extension(name)] || 'application/octet-stream'
}

/** "0:08", "1:02:03" */
export function formatDuration(seconds) {
  if (!seconds && seconds !== 0) return ''
  const s = Math.round(seconds)
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const rest = String(s % 60).padStart(2, '0')
  return h ? `${h}:${String(m).padStart(2, '0')}:${rest}` : `${m}:${rest}`
}

/**
 * Whether the browser can hand files to the system share sheet, which on iPhone and Android is
 * the way into the photo gallery ("Save 12 Images"). Desktop browsers mostly can't.
 */
export const canSaveToPhotos = (() => {
  try {
    return !!navigator.canShare?.({ files: [new File([''], 'photo.jpg', { type: 'image/jpeg' })] })
  } catch {
    return false
  }
})()

/**
 * The share's file URLs. `v` is the file's modification time, so thumbnails and previews can be
 * cached for good and still change when the file does.
 */
export function shareFileUrls(token, sessionToken, folderPath) {
  function url(endpoint, entry, extra = {}) {
    const path = folderPath() ? `${folderPath()}/${entry.name}` : entry.name
    const params = new URLSearchParams({ path, ...extra })
    if (sessionToken) params.set('session', sessionToken)
    return `/share/api/share/${token}/files/${endpoint}?${params}`
  }
  return {
    thumbnail: (entry, size = 320) => url('thumbnail', entry, { size: String(size), v: String(Math.floor(entry.modified)) }),
    preview: (entry) => url('preview', entry, { v: String(Math.floor(entry.modified)) }),
    poster: (entry) => url('poster', entry, { v: String(Math.floor(entry.modified)) }),
    view: (entry) => url('view', entry),
    download: (entry) => url('download', entry),
  }
}

/** Starts a browser download of `url` without loading it into memory first. */
export function downloadLink(url, filename) {
  const a = document.createElement('a')
  a.href = url
  a.download = filename || ''
  document.body.appendChild(a)
  a.click()
  a.remove()
}
