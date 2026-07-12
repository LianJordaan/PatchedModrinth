<script setup lang="ts">
import { useModalStack } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

const DEFAULT_URL = 'https://panel.bytebuilders.co.za'
// Custom panel URL (Settings -> Plugins). Empty falls back to the default.
const panelUrl = () => localStorage.getItem('bytelauncher-hosting-url')?.trim() || DEFAULT_URL

const container = ref<HTMLElement | null>(null)
const { hasModal } = useModalStack()
let resizeObserver: ResizeObserver | null = null

// The panel blocks <iframe> embedding (X-Frame-Options: DENY), so the backend
// renders it in a native child webview positioned exactly over `container`.
function show() {
	const el = container.value
	// A native webview always paints above HTML, so keep it hidden while a modal
	// is open — otherwise it would cover the modal.
	if (!el || hasModal.value) return
	const r = el.getBoundingClientRect()
	const dpr = window.devicePixelRatio || 1
	invoke('plugin:addons|set_hosting_webview', {
		visible: true,
		x: r.left * dpr,
		y: r.top * dpr,
		width: r.width * dpr,
		height: r.height * dpr,
		url: panelUrl(),
	}).catch((e) => console.error('[hosting] failed to show panel webview', e))
}

function hide() {
	invoke('plugin:addons|set_hosting_webview', {
		visible: false,
		x: 0,
		y: 0,
		width: 0,
		height: 0,
		url: panelUrl(),
	}).catch(() => {})
}

watch(hasModal, (open) => (open ? hide() : show()))

onMounted(() => {
	requestAnimationFrame(show)
	resizeObserver = new ResizeObserver(() => show())
	if (container.value) resizeObserver.observe(container.value)
	window.addEventListener('resize', show)
})

onBeforeUnmount(() => {
	resizeObserver?.disconnect()
	window.removeEventListener('resize', show)
	hide()
})
</script>

<template>
	<div class="flex h-full w-full flex-col">
		<div
			class="flex items-center justify-between gap-2 border-0 border-b border-solid border-surface-5 bg-surface-2 px-4 py-2"
		>
			<span class="text-sm font-semibold text-contrast">ByteBuilders Hosting</span>
			<button
				class="cursor-pointer border-none bg-transparent text-sm font-semibold text-brand hover:underline"
				@click="openUrl(panelUrl())"
			>
				Open in browser ↗
			</button>
		</div>
		<!-- The native ByteBuilders panel webview is positioned over this area by the backend. -->
		<div ref="container" class="min-h-0 w-full flex-1 bg-surface-1"></div>
	</div>
</template>
