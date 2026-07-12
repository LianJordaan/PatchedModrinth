<script setup lang="ts">
import { ButtonStyled, Toggle } from '@modrinth/ui'
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

import { openPath, restartApp } from '@/helpers/utils'
import { setPluginEnabledState } from '@/plugins/plugin-state'

interface PluginData {
	id: string
	name: string
	description: string
	version: string
	author: string
	enabled: boolean
	builtin: boolean
	js: string | null
	css: string | null
}

const plugins = ref<PluginData[]>((await invoke('plugin:addons|read_plugins')) as PluginData[])
const pluginsDir = ref<string>((await invoke('plugin:addons|get_plugins_dir')) as string)
const restartNeeded = ref(false)

async function toggle(plugin: PluginData) {
	const next = !plugin.enabled
	plugin.enabled = next
	restartNeeded.value = true
	setPluginEnabledState(plugin.id, next)
	try {
		await invoke('plugin:addons|set_plugin_enabled', { id: plugin.id, enabled: next })
	} catch (e) {
		console.error('Failed to toggle plugin', e)
		plugin.enabled = !next
		setPluginEnabledState(plugin.id, !next)
	}
}

async function openFolder() {
	if (pluginsDir.value) {
		await openPath(pluginsDir.value)
	}
}

const REPO = 'LianJordaan/ByteLauncher'
type UpdateState = 'idle' | 'checking' | 'current' | 'available' | 'installing' | 'error'
interface GithubAsset {
	name?: string
	browser_download_url?: string
	digest?: string | null
}
const currentVersion = ref<string>(await getVersion())
const updateState = ref<UpdateState>('idle')
const updateInfo = ref<{ version: string; url: string; sha256: string } | null>(null)
const updateError = ref<string>('')

// Compares versions like "0.15.10-fork.4" by their numeric parts so we only
// ever offer a strictly-newer release (never a downgrade).
function isNewer(remote: string, local: string): boolean {
	const parse = (v: string) => (v.match(/\d+/g) || []).map(Number)
	const a = parse(remote)
	const b = parse(local)
	for (let i = 0; i < Math.max(a.length, b.length); i++) {
		const x = a[i] ?? 0
		const y = b[i] ?? 0
		if (x !== y) return x > y
	}
	return false
}

async function checkForUpdates() {
	updateState.value = 'checking'
	updateError.value = ''
	try {
		const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
			headers: { Accept: 'application/vnd.github+json' },
		})
		if (!res.ok) throw new Error(`GitHub returned ${res.status}`)
		const data = await res.json()
		const latest = String(data.tag_name || '').replace(/^v/, '')
		// The standalone app binary, never an installer.
		const asset = (data.assets || []).find(
			(a: GithubAsset) =>
				/\.exe$/i.test(a?.name ?? '') && !/setup|installer/i.test(a?.name ?? ''),
		)
		if (latest && asset?.browser_download_url && isNewer(latest, currentVersion.value)) {
			updateInfo.value = {
				version: latest,
				url: asset.browser_download_url,
				sha256: (asset.digest ?? '').replace(/^sha256:/i, ''),
			}
			updateState.value = 'available'
		} else {
			updateState.value = 'current'
		}
	} catch (e) {
		updateError.value = e instanceof Error ? e.message : String(e)
		updateState.value = 'error'
	}
}

async function installUpdate() {
	if (!updateInfo.value) return
	updateState.value = 'installing'
	updateError.value = ''
	try {
		await invoke('plugin:addons|fork_apply_update', {
			downloadUrl: updateInfo.value.url,
			expectedSha256: updateInfo.value.sha256 || null,
		})
		// The app restarts on success, so this normally does not return.
	} catch (e) {
		updateError.value = e instanceof Error ? e.message : String(e)
		updateState.value = 'error'
	}
}

const confirmingUninstall = ref<boolean>(false)
const uninstallError = ref<string>('')

async function uninstall() {
	uninstallError.value = ''
	try {
		await invoke('plugin:addons|fork_uninstall')
		// The app quits on success so the uninstaller can run.
	} catch (e) {
		uninstallError.value = e instanceof Error ? e.message : String(e)
		confirmingUninstall.value = false
	}
}

const DEFAULT_HOSTING_URL = 'https://panel.bytebuilders.co.za'
const hostingUrl = ref<string>(localStorage.getItem('bytelauncher-hosting-url') ?? '')

function saveHostingUrl() {
	const value = hostingUrl.value.trim()
	if (value) {
		localStorage.setItem('bytelauncher-hosting-url', value)
	} else {
		localStorage.removeItem('bytelauncher-hosting-url')
	}
	invoke('plugin:addons|reload_hosting_webview', {
		url: value || DEFAULT_HOSTING_URL,
	}).catch(() => {})
}
</script>

<template>
	<div class="flex flex-col gap-4 min-w-[600px]">
		<div class="flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">App updates</h2>
				<p class="m-0 mt-1 text-sm">
					<span>You're on ByteLauncher v{{ currentVersion }} (based on Modrinth {{ currentVersion.split('-fork')[0] }}). </span>
					<span v-if="updateState === 'current'">You're up to date.</span>
					<span v-else-if="updateState === 'available'"
						>Update available: v{{ updateInfo?.version }}.</span
					>
					<span v-else-if="updateState === 'installing'"
						>Downloading and installing — the app will restart…</span
					>
					<span v-else-if="updateState === 'error'">Update failed: {{ updateError }}</span>
				</p>
			</div>
			<div class="flex items-center gap-2">
				<ButtonStyled v-if="updateState === 'available'" color="brand">
					<button :disabled="updateState === 'installing'" @click="installUpdate">
						Download &amp; install
					</button>
				</ButtonStyled>
				<ButtonStyled>
					<button
						:disabled="updateState === 'checking' || updateState === 'installing'"
						@click="checkForUpdates"
					>
						{{ updateState === 'checking' ? 'Checking…' : 'Check for updates' }}
					</button>
				</ButtonStyled>
			</div>
		</div>
		<div>
			<h2 class="m-0 text-lg font-semibold text-contrast">ByteBuilders Hosting</h2>
			<p class="m-0 mt-1 text-sm">
				The panel shown on the Hosting page. Leave empty to use the default ByteBuilders panel
				(<code>panel.bytebuilders.co.za</code>). Handy if you run a private panel.
			</p>
			<input
				v-model="hostingUrl"
				type="url"
				spellcheck="false"
				placeholder="https://panel.bytebuilders.co.za"
				class="mt-2 w-full rounded-lg border border-solid border-surface-5 bg-surface-1 px-3 py-2 text-sm text-contrast outline-none focus:border-brand"
				@change="saveHostingUrl"
				@blur="saveHostingUrl"
			/>
		</div>
		<div class="flex items-center justify-between gap-4">
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">Plugins</h2>
				<p class="m-0 mt-1 text-sm">
					Toggle plugins on or off. Plugins live in your plugins folder — add your own by dropping
					in a folder containing a <code>manifest.json</code> with its <code>.js</code>/<code>.css</code>.
				</p>
			</div>
			<ButtonStyled>
				<button @click="openFolder">Open plugins folder</button>
			</ButtonStyled>
		</div>

		<div v-if="restartNeeded" class="flex items-center justify-between gap-4">
			<p class="m-0 text-sm">Restart the app to apply your changes.</p>
			<ButtonStyled color="brand">
				<button @click="restartApp">Restart now</button>
			</ButtonStyled>
		</div>

		<div
			v-for="plugin in plugins"
			:key="plugin.id"
			class="flex items-center justify-between gap-4"
		>
			<div>
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ plugin.name }}
					<span v-if="plugin.builtin" class="text-sm font-normal">(built-in)</span>
				</h2>
				<p class="m-0 mt-1 text-sm">{{ plugin.description }}</p>
				<p v-if="plugin.version || plugin.author" class="m-0 mt-1 text-sm">
					<span v-if="plugin.version">v{{ plugin.version }}</span>
					<span v-if="plugin.author"> · {{ plugin.author }}</span>
				</p>
			</div>
			<Toggle
				:id="`plugin-${plugin.id}`"
				:model-value="plugin.enabled"
				@update:model-value="() => toggle(plugin)"
			/>
		</div>

		<p v-if="plugins.length === 0" class="m-0 text-sm">No plugins found.</p>

		<div class="mt-2 border-0 border-t border-solid border-surface-5 pt-6">
			<h2 class="m-0 text-lg font-semibold text-red">Danger zone</h2>
			<div class="mt-3 flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-base font-semibold text-contrast">Uninstall ByteLauncher</h3>
					<p class="m-0 mt-1 text-sm">
						Reverts to the Modrinth App — restores <code>Modrinth App.exe</code>, removes
						ByteLauncher and closes. Your instances, accounts and settings are kept.
					</p>
					<p v-if="uninstallError" class="m-0 mt-1 text-sm text-red">{{ uninstallError }}</p>
				</div>
				<div class="flex shrink-0 items-center gap-2">
					<ButtonStyled v-if="!confirmingUninstall" color="red">
						<button @click="confirmingUninstall = true">Uninstall</button>
					</ButtonStyled>
					<template v-else>
						<ButtonStyled>
							<button @click="confirmingUninstall = false">Cancel</button>
						</ButtonStyled>
						<ButtonStyled color="red">
							<button @click="uninstall">Yes, revert to Modrinth</button>
						</ButtonStyled>
					</template>
				</div>
			</div>
		</div>
	</div>
</template>
