<script lang="ts">
	import type { Camera } from '$lib/types';
	import { Eye, Trash2, Play, Pencil, MapPin, Layers, WifiOff, Video } from 'lucide-svelte';
	import { app } from '$lib/stores/app.svelte';
	import { onMount } from 'svelte';

	let {
		camera,
		onEdit,
		onDelete,
		onView,
		onScreenshot
	}: {
		camera: Camera;
		onEdit: (id: number) => void;
		onDelete: (id: number) => void;
		onView: (id: number) => void;
		onScreenshot?: (id: number) => void;
	} = $props();

	let status = $derived(app.cameraStatuses[camera.name] ?? (camera.enabled ? 'unknown' : 'disabled'));
	
	onMount(() => {
		// Load thumbnail on mount
		app.loadCameraThumbnail(camera.id);
	});
</script>

<!-- Mobile-first compact card with thumbnail -->
<div class="bg-surface-alt border border-edge rounded-2xl overflow-hidden transition-all hover:-translate-y-1 hover:shadow-xl group"
	style="box-shadow: 0 2px 8px var(--th-shadow);">
	
	<!-- Thumbnail Preview -->
	<div class="relative aspect-video bg-surface-raised overflow-hidden">
		{#if camera.thumbnail_url}
			<img 
				src={camera.thumbnail_url} 
				alt="{camera.name} preview" 
				class="w-full h-full object-cover"
			/>
		{:else}
			<div class="w-full h-full flex items-center justify-center">
				<Video class="w-12 h-12 text-content-muted/30" />
			</div>
		{/if}
		
		<!-- Status overlay -->
		<div class="absolute top-2 left-2">
			<div class="flex items-center gap-1.5 bg-surface-alt/90 backdrop-blur-sm px-2 py-1 rounded-lg text-xs font-medium">
				<span class="status-dot {status === 'online' ? 'on' : status === 'disabled' || status === 'offline' ? 'off' : 'unknown'}"></span>
				{#if !camera.enabled}
					<span class="text-content-muted">Deshabilitada</span>
				{:else if status === 'offline'}
					<span class="text-destructive flex items-center gap-1"><WifiOff class="w-3 h-3" /> Offline</span>
				{:else if status === 'online'}
					<span class="text-success">Online</span>
				{:else}
					<span class="text-content-muted">Comprobando...</span>
				{/if}
			</div>
		</div>
		
		<!-- Recording badge -->
		{#if camera.record}
			<div class="absolute top-2 right-2">
				<span class="badge badge-danger text-[10px] px-1.5 py-0.5">● REC</span>
			</div>
		{/if}
		
		<!-- Quick actions overlay (visible on hover) -->
		<div class="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-2">
			<button 
				onclick={() => onView(camera.id)} 
				class="bg-primary text-white px-4 py-2 rounded-lg font-medium flex items-center gap-2 hover:bg-primary-600 transition shadow-lg"
			>
				<Play class="w-4 h-4" /> Ver en vivo
			</button>
		</div>
	</div>
	
	<!-- Card content -->
	<div class="p-3">
		<!-- Title and actions -->
		<div class="flex items-start justify-between gap-2 mb-2">
			<h4 class="font-semibold text-sm text-content truncate flex-1">{camera.name}</h4>
			<div class="flex gap-1 shrink-0">
				<button onclick={() => onEdit(camera.id)} class="p-1 text-content-muted hover:text-amber-500 transition rounded" title="Editar">
					<Pencil class="w-3.5 h-3.5" />
				</button>
				<button onclick={() => onDelete(camera.id)} class="p-1 text-content-muted hover:text-destructive transition rounded" title="Eliminar">
					<Trash2 class="w-3.5 h-3.5" />
				</button>
			</div>
		</div>
		
		<!-- Location / Area badges -->
		{#if camera.location || camera.area}
			<div class="flex items-center gap-1.5 flex-wrap mb-2">
				{#if camera.location}
					<span class="badge badge-sm badge-info inline-flex items-center gap-1 text-[10px]">
						<MapPin class="w-2.5 h-2.5" /> {camera.location}
					</span>
				{/if}
				{#if camera.area}
					<span class="badge badge-sm badge-success inline-flex items-center gap-1 text-[10px]">
						<Layers class="w-2.5 h-2.5" /> {camera.area}
					</span>
				{/if}
			</div>
		{/if}
		
		<!-- Info line -->
		<div class="text-[11px] text-content-secondary truncate">
			{camera.host}:{camera.port}
		</div>
	</div>
</div>
