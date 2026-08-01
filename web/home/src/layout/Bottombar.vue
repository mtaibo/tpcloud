<script setup>

    import { ref, computed, watch, onMounted, onUnmounted } from 'vue'

    const props = defineProps({ activeSection: String, sections: Array })
    const emit = defineEmits(['update:activeSection'])

    const navRef      = ref(null)
    const hoverIndex  = ref(-1)
    const isDragging  = ref(false)
    const bubbleStyle = ref({ opacity: 0 })

    // Non-reactive drag state — not needed in template or computed
    let isPointerDown = false
    let startX = 0
    let startY = 0
    let directionLock = null
    let retryCount = 0
    let resizeObserver = null

    const visualIndex = computed(() =>
        isDragging.value && hoverIndex.value >= 0
            ? hoverIndex.value
            : props.sections.findIndex(t => t.id === props.activeSection)
    )

    function updateBubble() {
        const idx = visualIndex.value
        const nav = navRef.value
        if (idx < 0 || !nav) { bubbleStyle.value = { opacity: 0 }; return }

        const button = nav.querySelectorAll('.tab-item')[idx]
        if (!button) { bubbleStyle.value = { opacity: 0 }; return }

        const btnRect = button.getBoundingClientRect()
        if (btnRect.width === 0 && retryCount < 10) {
            retryCount++
            requestAnimationFrame(updateBubble)
            return
        }
        retryCount = 0

        const navRect = nav.getBoundingClientRect()
        bubbleStyle.value = {
            opacity:   isDragging.value ? 0.85 : 1,
            transform: `translateX(${btnRect.left - navRect.left}px) scale(${isDragging.value ? 1.5 : 1})`,
            width:     `${btnRect.width}px`,
            height:    `${btnRect.height}px`,
        }
    }

    watch([visualIndex, isDragging], updateBubble, { flush: 'post' })

    function activate(item) { emit('update:activeSection', item.id) }

    function getButtonIndex(nav, clientX) {
        return Array.from(nav.querySelectorAll('.tab-item')).findIndex(btn => {
            const { left, right } = btn.getBoundingClientRect()
            return clientX >= left && clientX < right
        })
    }

    function onPointerDown(e) {
        isPointerDown = true
        directionLock = null
        const touch = e.touches?.[0] || e
        startX = touch.clientX
        startY = touch.clientY
    }

    function onPointerMove(e) {
        if (!navRef.value || !isPointerDown) return
        const touch = e.touches?.[0] || e
        const deltaX = Math.abs(touch.clientX - startX)
        const deltaY = Math.abs(touch.clientY - startY)

        if (directionLock === null && (deltaX > 5 || deltaY > 5)) {
            directionLock = deltaY > deltaX ? 'vertical' : 'horizontal'
        }
        if (directionLock === 'vertical') return

        const index = getButtonIndex(navRef.value, touch.clientX)
        if (index >= 0) { hoverIndex.value = index; isDragging.value = true }
    }

    function onPointerEnd() {
        if (isDragging.value && hoverIndex.value >= 0) {
            const item = props.sections[hoverIndex.value]
            if (item) activate(item)
        }
        isDragging.value = false
        hoverIndex.value = -1
        isPointerDown = false
        directionLock = null
    }

    const listeners = [
        ['mousedown',   onPointerDown],
        ['touchstart',  onPointerDown, { passive: true }],
        ['mousemove',   onPointerMove],
        ['touchmove',   onPointerMove, { passive: true }],
        ['mouseup',     onPointerEnd],
        ['touchend',    onPointerEnd],
        ['touchcancel', onPointerEnd],
        ['mouseleave',  onPointerEnd],
    ]

    onMounted(() => {
        const nav = navRef.value
        if (!nav) return
        listeners.forEach(([event, handler, opts]) => nav.addEventListener(event, handler, opts))
        resizeObserver = new ResizeObserver(updateBubble)
        resizeObserver.observe(nav)
    })

    onUnmounted(() => {
        const nav = navRef.value
        if (nav) listeners.forEach(([event, handler, opts]) => nav.removeEventListener(event, handler, opts))
        resizeObserver?.disconnect()
    })

</script>

<template>

    <nav ref="navRef" class="liquid-bar" :class="{ dragging: isDragging }">
        <div class="bubble-indicator" :style="bubbleStyle"></div>
        <button
            v-for="(item, index) in props.sections"
            :key="item.id"
            @click="activate(item)"
            class="tab-item"
            :class="{ active: visualIndex === index }"
        >
            <component :is="item.icon" />
        </button>
    </nav>

</template>

<style scoped>
@reference "tailwindcss";

    .liquid-bar {
        @apply fixed left-4 right-4 z-50 rounded-[32px] px-1.5 py-1.5 flex items-center justify-between;
        bottom: 1.25rem;
        position: fixed;
        background: linear-gradient(
            135deg,
            rgba(255, 255, 255, 0.14) 0%,
            rgba(255, 255, 255, 0.10) 50%,
            rgba(255, 255, 255, 0.12) 100%
        );
        backdrop-filter: blur(60px) saturate(240%);
        -webkit-backdrop-filter: blur(60px) saturate(240%);
        border: 0.5px solid rgba(255, 255, 255, 0.25);
        box-shadow:
            inset 0 2px 0 rgba(255, 255, 255, 0.4),
            inset 0 -1px 0 rgba(0, 0, 0, 0.15),
            inset 1px 0 0 rgba(255, 255, 255, 0.12),
            inset -1px 0 0 rgba(255, 255, 255, 0.12),
            0 0 0 0.5px rgba(255, 255, 255, 0.15);
        transition: padding 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);

        @media (min-width: 768px) {
            display: none;
        }
    }

    .liquid-bar.dragging {
        padding-top: 7px;
        padding-bottom: 7px;
    }

    .bubble-indicator {
        position: absolute;
        top: 6px;
        left: 0;
        border-radius: 26px;
        background: linear-gradient(
            135deg,
            rgba(0, 0, 0, 0.4) 0%,
            rgba(0, 0, 0, 0.3) 100%
        );
        box-shadow:
            inset 0 1px 0 rgba(255, 255, 255, 0.1),
            inset 0 -0.5px 0 rgba(0, 0, 0, 0.3),
            0 4px 12px rgba(0, 0, 0, 0.5),
            0 0 0 0.5px rgba(255, 255, 255, 0.08);
        backdrop-filter: blur(20px);
        -webkit-backdrop-filter: blur(20px);
        transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
        pointer-events: none;
        z-index: 1;
    }

    .tab-item {
        @apply flex items-center justify-center h-[52px] rounded-[26px] cursor-pointer flex-1 transition-all duration-200;
        position: relative;
        z-index: 2;
    }

    .tab-item svg {
        @apply w-[24px] h-[24px];
        opacity: 1;
        color: #fff;
        filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.3));
        transition: all 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
    }

    .tab-item.active svg {
        color: var(--color-tp-accent);
        transform: scale(1.05);
    }

</style>
