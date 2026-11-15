import { BookImage, Eye, EyeOff, Heart, Library, Pencil, Plus, Trash2 } from 'lucide-vue-next'
import { confirmDestructive, prompt, toast } from '@ui/dialog.js'
import { t } from '@ui/i18n.js'

// Albums, managed the same way as in the iPhone app: make one, rename it, hide it (it moves to
// Hidden), keep its photos out of the library so they're only seen inside it, or delete it (its
// photos stay). Favoritos is everyone's and can't be changed.

const API = '/store/api/albums'

export const albumName = a => (a.kind === 'favorites' ? t('Favourites', 'Favoritos') : a.name)
export const albumIcon = a => (a.kind === 'favorites' ? Heart : a.hidden ? EyeOff : BookImage)

async function send(url, method, body) {
  const res = await fetch(url, {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  }).catch(() => null)
  if (!res?.ok) throw new Error()
  return res.json()
}

// A new album (empty, or with `photoIds`). Answers it, or null if cancelled or refused.
export async function createAlbum(photoIds = []) {
  const name = (await prompt({ title: t('New album', 'Nuevo álbum'), placeholder: t('Name', 'Nombre'), confirm: t('Create', 'Crear') }))?.trim()
  if (!name) return null
  try {
    return await send(API, 'POST', { name, photo_ids: photoIds })
  } catch {
    toast(t('The album couldn’t be created', 'No se ha podido crear el álbum'), 'warning')
    return null
  }
}

async function update(a, changes, done) {
  try {
    await send(`${API}/${a.id}`, 'PATCH', changes)
    if (done) toast(done)
    return true
  } catch {
    toast(t('The album couldn’t be changed', 'No se ha podido cambiar el álbum'), 'warning')
    return false
  }
}

// The menu for one album, in the sidebar, the phone's Albums menu and over an open album.
// `changed({ library })` runs after any change; `library` is true when the library's photos may
// have changed too (an album that keeps them out was changed or deleted). `deleted()` after a delete.
export function albumMenuItems(a, { changed, deleted }) {
  if (a.kind === 'favorites') return []
  return [
    {
      label: t('Rename…', 'Renombrar…'),
      icon: Pencil,
      action: async () => {
        const name = (await prompt({ title: t('Rename album', 'Renombrar álbum'), value: a.name, confirm: t('Rename', 'Renombrar') }))?.trim()
        if (name && name !== a.name && await update(a, { name })) changed({ library: false })
      },
    },
    {
      label: a.hidden ? t('Show album', 'Mostrar álbum') : t('Hide album', 'Ocultar álbum'),
      icon: a.hidden ? Eye : EyeOff,
      action: async () => {
        const ok = await update(a, { hidden: !a.hidden }, a.hidden ? t('Album shown', 'Álbum visible') : t('Moved to Hidden', 'Movido a Ocultos'))
        if (ok) changed({ library: false })
      },
    },
    {
      label: t('Keep out of the library', 'Mantener fuera de la biblioteca'),
      icon: Library,
      checked: a.hides_from_library,
      action: async () => {
        const hides = !a.hides_from_library
        const ok = await update(a, { hides_from_library: hides }, hides
          ? t('Its photos are only in the album now', 'Sus fotos ya solo están en el álbum')
          : t('Its photos are back in the library', 'Sus fotos vuelven a la biblioteca'))
        if (ok) changed({ library: true })
      },
    },
    { separator: true },
    {
      label: t('Delete album', 'Eliminar álbum'),
      icon: Trash2,
      danger: true,
      action: async () => {
        const ok = await confirmDestructive(
          t(`Delete “${a.name}”?`, `¿Eliminar «${a.name}»?`),
          t('The photos aren’t deleted: they stay in the library.', 'Las fotos no se borran: siguen en la biblioteca.'),
          t('Delete album', 'Eliminar álbum'),
        )
        if (!ok) return
        try {
          await send(`${API}/${a.id}`, 'DELETE')
          toast(t('Album deleted', 'Álbum eliminado'))
          deleted?.()
          changed({ library: a.hides_from_library })
        } catch {
          toast(t('The album couldn’t be deleted', 'No se ha podido eliminar el álbum'), 'warning')
        }
      },
    },
  ]
}

export const newAlbumItem = action => ({ label: t('New album…', 'Nuevo álbum…'), icon: Plus, action })
