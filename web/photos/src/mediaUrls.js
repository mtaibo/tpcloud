// URLs for a photo or video's light versions, like the share page's: a square thumbnail, the
// poster frame of a video, and the preview Quick Look shows (a 2048 px JPEG or a 720p MP4).
// `v` is the file's version, so the browser keeps them for good and still notices a change.

const THUMB_SIZE = 320

function url(endpoint, location, path, version, extra = {}) {
  const params = new URLSearchParams({ path, location, ...extra })
  if (version) params.set('v', String(version))
  return `/store/api/files/${endpoint}?${params}`
}

/** For a file in the store: `pathOf(entry)` gives its path, its version is its mtime. */
export function storeMediaUrls(location, pathOf) {
  const version = entry => (entry.modified ? Math.floor(entry.modified) : '')
  return {
    thumbnail: entry => url('thumbnail', location(), pathOf(entry), version(entry), { size: String(THUMB_SIZE) }),
    poster: entry => url('poster', location(), pathOf(entry), version(entry)),
    preview: entry => url('preview', location(), pathOf(entry), version(entry)),
    download: entry => url('download', location(), pathOf(entry)),
  }
}

/** For a link's files, seen from inside the store. */
export function shareMediaUrls(token, pathOf) {
  const u = (endpoint, entry, extra = {}) => {
    const params = new URLSearchParams({ path: pathOf(entry), ...extra })
    if (entry.modified) params.set('v', String(Math.floor(entry.modified)))
    return `/store/api/share/${token}/files/${endpoint}?${params}`
  }
  return {
    thumbnail: entry => u('thumbnail', entry, { size: String(THUMB_SIZE) }),
    poster: entry => u('poster', entry),
    preview: entry => u('preview', entry),
    download: entry => `/store/api/share/${token}/files/download?${new URLSearchParams({ path: pathOf(entry) })}`,
  }
}

/** For a backed-up photo: its checksum is its version, since a photo's file never changes. */
export const photoMediaUrls = {
  thumbnail: p => url('thumbnail', 'external', p.path, p.checksum, { size: String(THUMB_SIZE) }),
  poster: p => url('poster', 'external', p.path, p.checksum),
  preview: p => url('preview', 'external', p.path, p.checksum),
  download: p => url('download', 'external', p.path),
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

/** Starts a browser download of `url` without reading it into memory first. */
export function downloadLink(url, filename) {
  const a = document.createElement('a')
  a.href = url
  a.download = filename || ''
  document.body.appendChild(a)
  a.click()
  a.remove()
}
