<script setup>
import { computed } from 'vue'
import Icon from './Icon.vue'

// One row of a grouped list: an icon tile, a title with an optional subtitle, a trailing value
// and a chevron when it leads somewhere. It's a link with `href`/`to`, a button when it's
// clickable, and a plain row otherwise.
const props = defineProps({
  title: { type: String, default: '' },
  subtitle: { type: String, default: '' },
  value: { type: String, default: '' },
  icon: { type: String, default: '' },
  tint: { type: String, default: '' },
  plainIcon: { type: Boolean, default: false },
  href: { type: String, default: '' },
  to: { type: [String, Object], default: '' },
  clickable: { type: Boolean, default: false },
  chevron: { type: Boolean, default: undefined },
  destructive: { type: Boolean, default: false },
  accent: { type: Boolean, default: false },
  centered: { type: Boolean, default: false },
  disabled: { type: Boolean, default: false },
  smallSubtitle: { type: Boolean, default: false },
  wrapValue: { type: Boolean, default: false },
})
defineEmits(['click'])

const tag = computed(() => (props.to ? 'router-link' : props.href ? 'a' : props.clickable ? 'button' : 'div'))
// Only the attributes this element takes: RouterLink's own href must not be overwritten.
const linkAttrs = computed(() => {
  if (tag.value === 'router-link') return { to: props.to }
  if (tag.value === 'a') return { href: props.href }
  if (tag.value === 'button') return { type: 'button', disabled: props.disabled }
  return {}
})
const showChevron = computed(() => props.chevron ?? Boolean(props.to || props.href))
const iconStyle = computed(() => (props.tint && !props.plainIcon ? { background: `var(--${props.tint})` } : null))
</script>

<template>
  <component
    :is="tag"
    class="row"
    :class="{ destructive, accent, centered, 'no-icon': !icon && !$slots.leading }"
    v-bind="linkAttrs"
    @click="!disabled && $emit('click', $event)"
  >
    <slot name="leading">
      <span v-if="icon" class="row-icon" :class="{ plain: plainIcon }" :style="iconStyle">
        <Icon :name="icon" :size="plainIcon ? 24 : 19" :weight="plainIcon ? 1.8 : 2" />
      </span>
    </slot>
    <span v-if="title || subtitle || $slots.default" class="row-body">
      <slot>
        <span class="row-title">{{ title }}</span>
        <span v-if="subtitle" class="row-subtitle" :class="{ small: smallSubtitle }">{{ subtitle }}</span>
      </slot>
    </span>
    <slot name="trailing">
      <span v-if="value" class="row-value" :class="{ wrap: wrapValue }">{{ value }}</span>
    </slot>
    <Icon v-if="showChevron" name="chevron-right" class="row-chevron" :size="17" :weight="2.4" />
  </component>
</template>
