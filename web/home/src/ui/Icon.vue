<script setup>
// Line icons drawn for TPCloud in the weight of the system's symbols (24 pt grid, round caps).
const paths = {
  'chevron-right': '<path d="M9 5.5 15.5 12 9 18.5"/>',
  'chevron-left': '<path d="M15 5.5 8.5 12 15 18.5"/>',
  'chevron-down': '<path d="M5.5 9 12 15.5 18.5 9"/>',
  xmark: '<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>',
  checkmark: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  minus: '<path d="M5 12h14"/>',
  ellipsis: '<circle cx="5.5" cy="12" r="1.3" fill="currentColor" stroke="none"/><circle cx="12" cy="12" r="1.3" fill="currentColor" stroke="none"/><circle cx="18.5" cy="12" r="1.3" fill="currentColor" stroke="none"/>',
  person: '<circle cx="12" cy="8" r="3.8"/><path d="M4.5 20c.6-3.9 3.7-6.3 7.5-6.3s6.9 2.4 7.5 6.3"/>',
  'person-2': '<circle cx="9" cy="8.5" r="3.3"/><path d="M2.8 19.5c.5-3.4 3.1-5.5 6.2-5.5s5.7 2.1 6.2 5.5"/><path d="M15.5 5.6a3.2 3.2 0 0 1 0 6.1M17.5 14.3c2 .7 3.4 2.5 3.7 5.2"/>',
  key: '<circle cx="8" cy="15" r="4.5"/><path d="M11.3 11.7 19.5 3.5M16 7l2.5 2.5M18.2 4.8l2 2"/>',
  passkey: '<circle cx="9.5" cy="7.5" r="3.7"/><path d="M3 19.5c.5-3.6 3.1-5.8 6.5-5.8 1.1 0 2.1.2 3 .7"/><circle cx="17.5" cy="13.5" r="2.4"/><path d="M17.5 15.9v4.6M17.5 18.2h1.8"/>',
  lock: '<rect x="5" y="10.5" width="14" height="10" rx="2.5"/><path d="M8 10.5V8a4 4 0 0 1 8 0v2.5"/>',
  shield: '<path d="M12 3.2 19 6v5.6c0 4.4-2.9 8-7 9.4-4.1-1.4-7-5-7-9.4V6z"/><path d="M8.8 12.2l2.2 2.2 4.2-4.4"/>',
  qrcode: '<rect x="4" y="4" width="6" height="6" rx="1.2"/><rect x="14" y="4" width="6" height="6" rx="1.2"/><rect x="4" y="14" width="6" height="6" rx="1.2"/><path d="M14 14h2.5v2.5H14zM17.5 17.5H20V20h-2.5zM14 19.5V20M19.5 14h.5"/>',
  envelope: '<rect x="3" y="5.5" width="18" height="13" rx="2.5"/><path d="m3.8 7 8.2 6 8.2-6"/>',
  laptop: '<rect x="5" y="5" width="14" height="10" rx="1.6"/><path d="M2.5 18.5h19"/>',
  desktop: '<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M9 20h6M12 16v4"/>',
  iphone: '<rect x="7" y="2.8" width="10" height="18.4" rx="2.6"/><path d="M10.8 5.2h2.4"/>',
  ipad: '<rect x="4.5" y="3" width="15" height="18" rx="2.4"/><path d="M11 18.3h2"/>',
  app: '<rect x="4" y="4" width="16" height="16" rx="4.5"/><path d="M8.5 9.5h7M10 12.5a2 2 0 1 0 4 0 2 2 0 1 0-4 0M12 16.5h0"/>',
  globe: '<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.3 2.4 3.4 5.2 3.4 8.5s-1.1 6.1-3.4 8.5c-2.3-2.4-3.4-5.2-3.4-8.5S9.7 5.9 12 3.5z"/>',
  location: '<path d="M12 21s-6.5-5.6-6.5-11a6.5 6.5 0 0 1 13 0c0 5.4-6.5 11-6.5 11z"/><circle cx="12" cy="10" r="2.3"/>',
  clock: '<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>',
  calendar: '<rect x="4" y="5" width="16" height="15" rx="2.5"/><path d="M4 9.5h16M8.5 3v4M15.5 3v4"/>',
  'arrow-out': '<path d="M14 4h6v6M20 4l-8.5 8.5M18 14v3.5a2.5 2.5 0 0 1-2.5 2.5h-9A2.5 2.5 0 0 1 4 17.5v-9A2.5 2.5 0 0 1 6.5 6H10"/>',
  'sign-out': '<path d="M14.5 4.5H17a2.5 2.5 0 0 1 2.5 2.5v10a2.5 2.5 0 0 1-2.5 2.5h-2.5M10 8l-4 4 4 4M6 12h9.5"/>',
  house: '<path d="M4 10.5 12 4l8 6.5V19a1.5 1.5 0 0 1-1.5 1.5H15v-5.5H9v5.5H5.5A1.5 1.5 0 0 1 4 19z"/>',
  folder: '<path d="M3.5 7.5A2 2 0 0 1 5.5 5.5h3.6l2 2h7.4a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>',
  photo: '<rect x="3.5" y="5" width="17" height="14" rx="2.5"/><circle cx="9" cy="10" r="1.6"/><path d="m4.5 17.5 4.8-4.6 3.4 3.2 2.6-2.4 4.7 4.3"/>',
  server: '<rect x="4" y="4" width="16" height="7" rx="2"/><rect x="4" y="13" width="16" height="7" rx="2"/><path d="M8 7.5h.01M8 16.5h.01"/>',
  gear: '<circle cx="12" cy="12" r="3"/><path d="M12 3.5v2.2M12 18.3v2.2M20.5 12h-2.2M5.7 12H3.5M18 6l-1.6 1.6M7.6 16.4 6 18M18 18l-1.6-1.6M7.6 7.6 6 6"/>',
  trash: '<path d="M4.5 7h15M9.5 7V4.8h5V7M6.5 7l.9 12.2A1.8 1.8 0 0 0 9.2 21h5.6a1.8 1.8 0 0 0 1.8-1.8L17.5 7M10 11v6M14 11v6"/>',
  pencil: '<path d="M4 20h4L19 9l-4-4L4 16z"/><path d="m13.5 6.5 4 4"/>',
  info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.8h.01"/>',
  warning: '<path d="M12 4 21 19.5H3z"/><path d="M12 10v4.5M12 17h.01"/>',
  'check-circle': '<circle cx="12" cy="12" r="8.5"/><path d="m8 12.3 2.8 2.8L16.2 9.5"/>',
  'x-circle': '<circle cx="12" cy="12" r="8.5"/><path d="m9 9 6 6M15 9l-6 6"/>',
  history: '<path d="M4 12a8 8 0 1 0 2.4-5.7L4 8.7"/><path d="M4 4.5v4.2h4.2M12 8v4.3l2.8 1.7"/>',
  network: '<circle cx="12" cy="5.5" r="2"/><circle cx="5.5" cy="18.5" r="2"/><circle cx="18.5" cy="18.5" r="2"/><path d="M12 7.5v4M12 11.5l-5.2 5.4M12 11.5l5.2 5.4"/>',
  cloud: '<path d="M7 18.5h10.5a4 4 0 0 0 .6-8 5.5 5.5 0 0 0-10.6-1.3A4.7 4.7 0 0 0 7 18.5z"/>',
  bolt: '<path d="M13 3 5.5 13.5H12L11 21l7.5-10.5H12z"/>',
  terminal: '<rect x="3.5" y="4.5" width="17" height="15" rx="2.5"/><path d="m7.5 9.5 3 2.5-3 2.5M12.5 15h4"/>',
  faceid: '<path d="M4 8.5V6.5A2.5 2.5 0 0 1 6.5 4h2M15.5 4h2A2.5 2.5 0 0 1 20 6.5v2M20 15.5v2a2.5 2.5 0 0 1-2.5 2.5h-2M8.5 20h-2A2.5 2.5 0 0 1 4 17.5v-2M9 9.5v1.5M15 9.5v1.5M12 9.5v3.5h-1M9.5 16a3.5 3.5 0 0 0 5 0"/>',
  refresh: '<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3l2.2 2.1"/><path d="M19.5 4.5v4.3h-4.3"/>',
}

const props = defineProps({
  name: { type: String, required: true },
  size: { type: [Number, String], default: 22 },
  weight: { type: [Number, String], default: 1.8 },
})
</script>

<template>
  <svg
    class="icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="weight"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    v-html="paths[props.name] || ''"
  />
</template>
