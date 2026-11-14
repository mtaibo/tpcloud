import { t } from '@ui/i18n.js'
export const IMAGE_EXTS = new Set(['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp', 'tiff', 'heic'])
export const VIDEO_EXTS = new Set(['mp4', 'mkv', 'avi', 'mov', 'webm', 'flv', 'm4v'])
export const AUDIO_EXTS = new Set(['mp3', 'wav', 'flac', 'ogg', 'aac', 'm4a', 'opus'])
export const ARCHIVE_EXTS = new Set(['zip', 'tar', 'gz', 'bz2', '7z', 'rar', 'xz', 'zst'])
export const CODE_EXTS = new Set(['js', 'ts', 'py', 'go', 'rs', 'java', 'c', 'cpp', 'h', 'css', 'html', 'json', 'yaml', 'yml', 'sh', 'bash', 'zsh', 'toml', 'xml', 'vue', 'jsx', 'tsx', 'swift'])
const SHEET_EXTS = new Set(['csv', 'xls', 'xlsx', 'numbers', 'ods'])
const SLIDES_EXTS = new Set(['ppt', 'pptx', 'key', 'odp'])
const WORD_EXTS = new Set(['doc', 'docx', 'pages', 'odt', 'rtf'])
const TEXT_EXTS = new Set(['txt', 'md'])

export function extOf(entry) {
  const name = entry.name || ''
  return name.includes('.') ? name.split('.').pop().toLowerCase() : ''
}

/** What an entry is, for its icon, its colour and the "Tipo" column. */
export function kindOf(entry) {
  if (entry.type === 'upload-pending') return 'upload'
  if (entry.type === 'share-link') return 'share'
  if (entry.type === 'directory') return 'folder'
  const ext = extOf(entry)
  if (IMAGE_EXTS.has(ext)) return 'image'
  if (VIDEO_EXTS.has(ext)) return 'video'
  if (AUDIO_EXTS.has(ext)) return 'audio'
  if (ARCHIVE_EXTS.has(ext)) return 'archive'
  if (ext === 'pdf') return 'pdf'
  if (SHEET_EXTS.has(ext)) return 'sheet'
  if (SLIDES_EXTS.has(ext)) return 'slides'
  if (WORD_EXTS.has(ext)) return 'word'
  if (TEXT_EXTS.has(ext)) return 'text'
  if (CODE_EXTS.has(ext)) return 'code'
  return 'file'
}

// Read through kindName(), so the language can change while the page is open.
const KIND_NAMES = () => ({
  upload: t('Uploading', 'Subiendo'),
  share: t('Shared link', 'Enlace compartido'),
  folder: t('Folder', 'Carpeta'),
  image: t('Image', 'Imagen'),
  video: t('Video', 'Vídeo'),
  audio: 'Audio',
  archive: t('Archive', 'Archivo comprimido'),
  pdf: t('PDF document', 'Documento PDF'),
  sheet: t('Spreadsheet', 'Hoja de cálculo'),
  slides: t('Presentation', 'Presentación'),
  word: t('Document', 'Documento'),
  text: t('Text', 'Texto'),
  code: t('Source code', 'Código fuente'),
  file: t('Document', 'Documento'),
})

export function kindName(entry) {
  const kind = kindOf(entry)
  const ext = extOf(entry)
  if ((kind === 'image' || kind === 'video' || kind === 'audio') && ext) return `${KIND_NAMES()[kind]} ${ext.toUpperCase()}`
  return KIND_NAMES()[kind]
}

// The colour of a document's label, like the system's document icons.
export const KIND_COLORS = {
  image: 'var(--purple)',
  video: 'var(--pink)',
  audio: 'var(--red)',
  archive: 'var(--gray)',
  pdf: 'var(--red)',
  sheet: 'var(--green)',
  slides: 'var(--orange)',
  word: 'var(--blue)',
  text: 'var(--gray)',
  code: 'var(--indigo)',
  file: 'var(--gray)',
}

export function isMedia(entry) {
  const kind = kindOf(entry)
  return kind === 'image' || kind === 'video'
}
