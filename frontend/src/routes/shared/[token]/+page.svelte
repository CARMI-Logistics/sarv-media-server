<script lang="ts">
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import type { ShareAccess } from '$lib/types';
	import { brand } from '$lib/brand.config';
	import { AlertTriangle, Clock, Calendar, Video, Loader2 } from 'lucide-svelte';

	let token = $derived(page.params.token);
	let loading = $state(true);
	let errorMsg = $state('');
	let access = $state<ShareAccess | null>(null);

	// Backend URL - usar variable de entorno o fallback
	const API_URL = import.meta.env.VITE_API_URL || (import.meta.env.DEV ? 'http://localhost:8080' : '');

	onMount(async () => {
		await loadShare();
	});

	async function loadShare() {
		loading = true;
		errorMsg = '';
		try {
			const res = await fetch(`${API_URL}/share/${token}`);
			const json = await res.json();
			if (json.success) {
				access = json.data;
			} else {
				errorMsg = json.error || 'Enlace no válido';
			}
		} catch {
			errorMsg = 'Error de conexión';
		}
		loading = false;
	}

	let webrtcUrl = $derived(
		access ? `${globalThis.location?.protocol}//${globalThis.location?.hostname}:8889/${access.stream_name}` : ''
	);
	let hlsUrl = $derived(
		access ? `${globalThis.location?.protocol}//${globalThis.location?.hostname}:8888/${access.stream_name}/index.m3u8` : ''
	);

	let expiresFormatted = $derived(
		access?.expires_at ? new Date(access.expires_at).toLocaleString() : ''
	);
</script>

<svelte:head>
	<title>Mosaico Compartido - {brand.name}</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<div class="min-h-screen bg-surface flex flex-col">
	<!-- Header -->
	<header class="bg-surface-alt border-b border-edge px-4 py-3 flex items-center gap-3 safe-top" style="box-shadow: 0 1px 3px var(--th-shadow);">
		<Video class="w-5 h-5 text-primary" />
		<h1 class="text-sm font-bold text-content">{brand.name}</h1>
		{#if access}
			<span class="text-xs text-content-muted ml-auto">Mosaico: {access.mosaic_name}</span>
		{/if}
	</header>

	<!-- Content -->
	<main class="flex-1 flex items-center justify-center p-4">
		{#if loading}
			<div class="text-center space-y-3">
				<Loader2 class="w-10 h-10 text-primary mx-auto animate-spin" />
				<p class="text-sm text-content-muted">Verificando enlace...</p>
			</div>
		{:else if errorMsg}
			<div class="max-w-md w-full bg-surface-alt border border-edge rounded-2xl p-6 sm:p-8 text-center space-y-4" style="box-shadow: 0 4px 16px var(--th-shadow);">
				<div class="w-16 h-16 rounded-full mx-auto flex items-center justify-center bg-red-100 dark:bg-red-900/30">
					<AlertTriangle class="w-8 h-8 text-red-500" />
				</div>
				<h2 class="text-lg font-semibold text-content">Acceso denegado</h2>
				<p class="text-sm text-content-secondary">{errorMsg}</p>
				<button onclick={loadShare} class="btn btn-secondary py-2 text-sm mt-2">Reintentar</button>
			</div>
		{:else if access}
			<div class="max-w-4xl w-full space-y-4">
				<!-- Info bar -->
				<div class="bg-surface-alt border border-edge rounded-xl p-3 sm:p-4 flex flex-wrap items-center gap-4 text-xs text-content-muted" style="box-shadow: 0 1px 3px var(--th-shadow);">
					<span class="font-medium text-content text-sm">{access.mosaic_name}</span>
					<span class="flex items-center gap-1"><Clock class="w-3 h-3" /> Expira: {expiresFormatted}</span>
					{#if access.schedule_start && access.schedule_end}
						<span class="flex items-center gap-1"><Calendar class="w-3 h-3" /> Horario: {access.schedule_start} - {access.schedule_end}</span>
					{/if}
					{#if !access.is_active}
						<span class="badge badge-warn">Mosaico detenido</span>
					{/if}
				</div>

				{#if access.is_active}
					<!-- Video player -->
					<div class="bg-black rounded-xl overflow-hidden aspect-video relative">
						<iframe
							src={webrtcUrl}
							title="Mosaico {access.mosaic_name}"
							class="w-full h-full border-0"
							allow="autoplay"
						></iframe>
					</div>

					<!-- Alternative links -->
					<div class="flex flex-wrap gap-3 text-xs text-content-muted">
						<a href={webrtcUrl} target="_blank" rel="noopener" class="text-primary hover:underline flex items-center gap-1">
							<Video class="w-3 h-3" /> WebRTC
						</a>
						<a href={hlsUrl} target="_blank" rel="noopener" class="text-primary hover:underline flex items-center gap-1">
							<Video class="w-3 h-3" /> HLS
						</a>
					</div>
				{:else}
					<div class="bg-surface-alt border border-edge rounded-xl p-8 text-center space-y-3">
						<Video class="w-12 h-12 mx-auto text-content-muted/40" />
						<p class="text-sm text-content-muted">El mosaico no está activo actualmente</p>
						<p class="text-xs text-content-muted">Contacta al administrador para iniciar el mosaico</p>
					</div>
				{/if}
			</div>
		{/if}
	</main>

	<!-- Footer -->
	<footer class="text-center py-3 text-xs text-content-muted border-t border-edge">
		{brand.name} &middot; Enlace compartido con acceso restringido
	</footer>
</div>
