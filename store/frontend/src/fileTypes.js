import { Folder, FolderSymlink, File, FileText, Image, Film, Music, Archive, Code } from 'lucide-vue-next'

export const IMAGE_EXTS = new Set(['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp', 'tiff', 'heic'])
export const VIDEO_EXTS = new Set(['mp4', 'mkv', 'avi', 'mov', 'webm', 'flv', 'm4v'])
export const AUDIO_EXTS = new Set(['mp3', 'wav', 'flac', 'ogg', 'aac', 'm4a', 'opus'])
export const ARCHIVE_EXTS = new Set(['zip', 'tar', 'gz', 'bz2', '7z', 'rar', 'xz', 'zst'])
export const CODE_EXTS = new Set(['js', 'ts', 'py', 'go', 'rs', 'java', 'c', 'cpp', 'h', 'css', 'html', 'json', 'yaml', 'yml', 'sh', 'bash', 'zsh', 'toml', 'xml', 'vue', 'jsx', 'tsx'])
export const DOC_EXTS = new Set(['pdf', 'txt', 'md', 'doc', 'docx', 'odt', 'rtf', 'csv', 'xls', 'xlsx'])

export function getFileIcon(entry) {
  if (entry.type === 'share-link') return FolderSymlink
  if (entry.type === 'directory') return Folder
  const ext = (entry.name.split('.').pop() || '').toLowerCase()
  if (IMAGE_EXTS.has(ext)) return Image
  if (VIDEO_EXTS.has(ext)) return Film
  if (AUDIO_EXTS.has(ext)) return Music
  if (ARCHIVE_EXTS.has(ext)) return Archive
  if (CODE_EXTS.has(ext)) return Code
  if (DOC_EXTS.has(ext)) return FileText
  return File
}

export function getIconColor(entry) {
  if (entry.type === 'share-link') return '#007AFF'
  return entry.type === 'directory' ? '#007AFF' : '#636366'
}
