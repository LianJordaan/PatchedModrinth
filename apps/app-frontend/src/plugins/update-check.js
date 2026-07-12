import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'

// ByteLauncher: checks the fork's latest GitHub release on startup and shows
// a dismissible banner when a newer version is available. The banner installs
// the update in place (download -> verify -> swap -> relaunch) via the same
// `fork_apply_update` command the Settings page uses. It is fully self-contained
// (plain DOM) so it cannot interfere with Vue rendering.

const REPO = 'LianJordaan/ByteLauncher'
const BANNER_ID = 'mr-upstream-update-banner'
const DISMISS_KEY = 'mr-dismissed-update-version'

function showBanner(version, releaseUrl, asset) {
	if (document.getElementById(BANNER_ID)) return
	if (localStorage.getItem(DISMISS_KEY) === version) return

	const bar = document.createElement('div')
	bar.id = BANNER_ID
	bar.style.cssText = [
		'position:fixed',
		'left:0',
		'right:0',
		'bottom:0',
		'z-index:9998',
		'display:flex',
		'align-items:center',
		'justify-content:center',
		'gap:12px',
		'padding:8px 16px',
		'font-size:13px',
		'background:var(--color-raised-bg, #27292e)',
		'color:var(--color-contrast, #fff)',
		'border-top:1px solid var(--color-divider, #34363c)',
	].join(';')

	const text = document.createElement('span')
	text.textContent = `ByteLauncher ${version} is available.`

	const action = document.createElement('button')
	action.style.cssText = [
		'padding:4px 12px',
		'border:none',
		'border-radius:6px',
		'cursor:pointer',
		'font-weight:600',
		'background:var(--color-brand, #6c4bff)',
		'color:#fff',
	].join(';')

	if (asset) {
		// One-click update: download the new exe, verify it, swap it in and
		// restart — identical to Settings -> Plugins -> "Download & install".
		action.textContent = 'Download & install'
		action.addEventListener('click', async () => {
			action.disabled = true
			action.style.opacity = '0.6'
			action.style.cursor = 'default'
			dismiss.style.display = 'none'
			text.textContent = `Downloading ByteLauncher ${version} — the app will restart…`
			try {
				await invoke('plugin:addons|fork_apply_update', {
					downloadUrl: asset.url,
					expectedSha256: asset.sha256 || null,
				})
				// On success the app restarts, so this normally does not return.
			} catch (e) {
				action.disabled = false
				action.style.opacity = '1'
				action.style.cursor = 'pointer'
				dismiss.style.display = ''
				text.textContent = `Update failed: ${e instanceof Error ? e.message : String(e)}`
				console.error('[update-check] install failed', e)
			}
		})
	} else {
		// Fallback when we can't verify the asset (no published digest): send the
		// user to the release page rather than installing an unverified binary.
		action.textContent = 'View release'
		action.addEventListener('click', () => {
			openUrl(releaseUrl).catch((e) => console.error('[update-check] failed to open release', e))
		})
	}

	const dismiss = document.createElement('button')
	dismiss.textContent = '✕'
	dismiss.setAttribute('aria-label', 'Dismiss')
	dismiss.style.cssText = [
		'padding:4px 8px',
		'border:none',
		'border-radius:6px',
		'cursor:pointer',
		'background:transparent',
		'color:inherit',
		'font-size:14px',
	].join(';')
	dismiss.addEventListener('click', () => {
		localStorage.setItem(DISMISS_KEY, version)
		bar.remove()
	})

	bar.append(text, action, dismiss)
	document.body.appendChild(bar)
}

export async function checkForUpdates() {
	try {
		const current = await getVersion()
		// Skip local/dev/canary builds where the version is not a real release.
		if (/local|dev|canary/i.test(current)) return

		const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
			headers: { Accept: 'application/vnd.github+json' },
		})
		if (!res.ok) return

		const data = await res.json()
		const latest = String(data.tag_name || '').replace(/^v/, '')
		if (!latest || latest === current) return

		// The standalone app binary, never an installer.
		const exe = (data.assets || []).find(
			(a) => /\.exe$/i.test(a?.name ?? '') && !/setup|installer/i.test(a?.name ?? ''),
		)
		const sha256 = (exe?.digest ?? '').replace(/^sha256:/i, '')
		const asset =
			exe?.browser_download_url && sha256 ? { url: exe.browser_download_url, sha256 } : null

		showBanner(latest, data.html_url || `https://github.com/${REPO}/releases/latest`, asset)
	} catch {
		// Offline or rate-limited — silently skip.
	}
}
