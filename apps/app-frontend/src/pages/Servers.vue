<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { onBeforeUnmount, onMounted, ref } from 'vue'

const PANEL_URL = 'https://panel.bytebuilders.co.za'
const container = ref<HTMLElement | null>(null)
let resizeObserver: ResizeObserver | null = null

// The panel blocks <iframe> embedding (X-Frame-Options: DENY), so instead the
// backend renders it in a native child webview positioned exactly over the
// `container` element below. We just keep the backend informed of its bounds.
function show() {
	const el = container.value
	if (!el) return
	const r = el.getBoundingClientRect()
	const dpr = window.devicePixelRatio || 1
	invoke('plugin:addons|set_hosting_webview', {
		visible: true,
		x: r.left * dpr,
		y: r.top * dpr,
		width: r.width * dpr,
		height: r.height * dpr,
	}).catch((e) => console.error('[hosting] failed to show panel webview', e))
}

function hide() {
	invoke('plugin:addons|set_hosting_webview', {
		visible: false,
		x: 0,
		y: 0,
		width: 0,
		height: 0,
	}).catch(() => {})
}

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
				@click="openUrl(PANEL_URL)"
			>
				Open in browser ↗
			</button>
		</div>
		<!-- The native ByteBuilders panel webview is positioned over this area by the backend. -->
		<div ref="container" class="min-h-0 w-full flex-1 bg-surface-1"></div>
	</div>
</template>
