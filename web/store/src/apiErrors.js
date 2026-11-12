import { lang } from './ui/i18n.js'

// store-api answers in English; the messages people actually run into, in Spanish too.
const ES = {
  'File not found': 'No se encuentra el archivo',
  'Not found': 'No se encuentra',
  'Directory not found': 'No se encuentra la carpeta',
  'Permission denied': 'No tienes permiso',
  'Access denied': 'Acceso denegado',
  'Already exists': 'Ya existe',
  'Item already exists at destination': 'Ya hay un elemento con ese nombre en el destino',
  'Invalid name': 'Nombre no válido',
  'Invalid filename': 'Nombre de archivo no válido',
  'Invalid path': 'Ruta no válida',
  'Path is not a directory': 'No es una carpeta',
  'Destination is not a directory': 'El destino no es una carpeta',
  'Source not found': 'No se encuentra el original',
  'Cannot move a folder into itself': 'No se puede mover una carpeta dentro de sí misma',
  'Cannot delete the shared folder': 'No se puede eliminar la carpeta compartida',
  'Cannot delete root': 'No se puede eliminar la carpeta raíz',
  'Zip file already exists': 'Ya existe un ZIP con ese nombre',
  'Not a zip file': 'No es un archivo ZIP',
  'Not a valid ZIP file': 'No es un ZIP válido',
  'Invalid or corrupted zip file': 'El ZIP no es válido o está dañado',
  'Zip contains unsafe paths': 'El ZIP contiene rutas no seguras',
  'Slug already in use': 'Esa dirección ya está en uso',
  'Slug must be 1–64 alphanumeric/dash/underscore characters': 'La dirección debe tener de 1 a 64 letras, números, guiones o guiones bajos',
  'Share not found': 'No se encuentra el enlace',
  'Backup already running': 'Ya hay una copia en curso',
  'Admin access required': 'Hace falta ser administrador',
  'Checksum mismatch - uploaded file is corrupt, re-upload it': 'El archivo subido está dañado; vuelve a subirlo',
}

export const apiError = detail => (detail && lang.value === 'es' && ES[detail]) || detail
