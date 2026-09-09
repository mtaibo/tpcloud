<script setup>
import { computed } from 'vue'
import { HardDrive, Server, Folder } from 'lucide-vue-next'

const props = defineProps({
  user: Object,
  location: String,
  path: String,
})

const emit = defineEmits(['navigate'])

const crumbs = computed(() => {
  const isAdmin = props.user?.is_admin
  const loc = props.location
  const parts = (props.path || '').split('/').filter(Boolean)
  const items = []

  if (isAdmin) {
    // Root: Disk or System
    items.push({
      label: loc === 'external' ? 'Disk' : 'System',
      icon: loc === 'external' ? 'disk' : 'system',
      navigateLoc: loc,
      navigatePath: '',
    })

    // Path segments, collapsing users/email into "Personal"
    let i = 0
    while (i < parts.length) {
      if (loc === 'external' && parts[i] === 'users' && parts[i + 1] === props.user?.email) {
        items.push({
          label: 'Personal',
          icon: 'folder',
          navigateLoc: loc,
          navigatePath: parts.slice(0, i + 2).join('/'),
        })
        i += 2
      } else {
        items.push({
          label: parts[i],
          icon: 'folder',
          navigateLoc: loc,
          navigatePath: parts.slice(0, i + 1).join('/'),
        })
        i++
      }
    }
  } else {
    // Non-admin: root is Personal or Shared, no Disk prefix
    if (parts[0] === 'users' && parts[1] === props.user?.email) {
      items.push({
        label: 'Personal',
        icon: 'folder',
        navigateLoc: 'external',
        navigatePath: `users/${props.user.email}`,
      })
      for (let i = 2; i < parts.length; i++) {
        items.push({
          label: parts[i],
          icon: 'folder',
          navigateLoc: 'external',
          navigatePath: parts.slice(0, i + 1).join('/'),
        })
      }
    } else if (parts[0] === 'shared') {
      items.push({
        label: 'Shared',
        icon: 'folder',
        navigateLoc: 'external',
        navigatePath: 'shared',
      })
      for (let i = 1; i < parts.length; i++) {
        items.push({
          label: parts[i],
          icon: 'folder',
          navigateLoc: 'external',
          navigatePath: parts.slice(0, i + 1).join('/'),
        })
      }
    }
  }

  return items
})

function goTo(crumb) {
  emit('navigate', crumb.navigateLoc, crumb.navigatePath)
}
</script>

<template>
  <div class="breadcrumb">
    <template v-for="(crumb, i) in crumbs" :key="i">
      <span v-if="i > 0" class="sep">›</span>
      <button class="seg" :class="{ 'root-seg': i === 0 }" @click="goTo(crumb)">
        <HardDrive v-if="crumb.icon === 'disk'" class="seg-icon" />
        <Server v-else-if="crumb.icon === 'system'" class="seg-icon" />
        <Folder v-else class="seg-icon" />
        <span>{{ crumb.label }}</span>
      </button>
    </template>
  </div>
</template>

<style scoped>
.breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.2rem;
  font-size: 0.85rem;
  min-width: 0;
  overflow: hidden;
}

.seg {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  background: none;
  border: none;
  color: #737373;
  cursor: pointer;
  transition: color 0.15s;
  max-width: 8rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 1;
  padding: 0;
}

.root-seg { flex-shrink: 0; }

.seg:hover { color: #fff; }

.seg-icon { width: 14px; height: 14px; flex-shrink: 0; }

.sep {
  color: #404040;
  flex-shrink: 0;
  padding: 0 0.1rem;
}
</style>
