<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { app } from '$lib/stores/app.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import type { Camera, Location, AreaWithLocation, Mosaic, Tab } from '$lib/types';
	import CameraCard from '$lib/components/CameraCard.svelte';
	import CameraModal from '$lib/components/CameraModal.svelte';
	import CameraViewer from '$lib/components/CameraViewer.svelte';
	import LocationModal from '$lib/components/LocationModal.svelte';
	import AreaModal from '$lib/components/AreaModal.svelte';
	import MosaicModal from '$lib/components/MosaicModal.svelte';
	import {
		Video, Grid3x3, MapPin, Layers, RefreshCw, BookOpen, LogOut,
		Plus, Search, CameraOff, LayoutGrid, MapPinOff, Edit2, Trash2,
		Lock, Eye, Pencil, Play, Square, Tv, Radio
	} from 'lucide-svelte';

	// Modal state
	let cameraModalOpen = $state(false);
	let editCamera = $state<Camera | null>(null);
	let locationModalOpen = $state(false);
	let editLocation = $state<Location | null>(null);
	let areaModalOpen = $state(false);
	let editArea = $state<AreaWithLocation | null>(null);
	let mosaicModalOpen = $state(false);
	let editMosaic = $state<Mosaic | null>(null);
	let viewerOpen = $state(false);
	let viewerStreamName = $state('');

	let searchTimeout: ReturnType<typeof setTimeout>;

	onMount(() => {
		if (!auth.isAuthenticated) {
			goto('/login');
			return;
		}
		app.loadAll();
	});

	function switchTab(tab: Tab) {
		app.activeTab = tab;
	}

	// Camera actions
	function openNewCamera() {
		editCamera = null;
		cameraModalOpen = true;
	}
	function openEditCamera(id: number) {
		editCamera = app.cameras.find((c) => c.id === id) || null;
		cameraModalOpen = true;
	}
	async function confirmDeleteCamera(id: number) {
		const cam = app.cameras.find((c) => c.id === id);
		if (!cam) return;
		if (!confirm(`¿Eliminar la cámara "${cam.name}"?`)) return;
		await app.deleteCamera(id);
	}
	function openViewCamera(id: number) {
		const cam = app.cameras.find((c) => c.id === id);
		if (!cam) return;
		viewerStreamName = cam.name;
		viewerOpen = true;
	}

	// Location actions
	function openNewLocation() {
		editLocation = null;
		locationModalOpen = true;
	}
	function openEditLocation(id: number) {
		editLocation = app.locations.find((l) => l.id === id) || null;
		locationModalOpen = true;
	}
	async function confirmDeleteLocation(id: number) {
		const loc = app.locations.find((l) => l.id === id);
		if (!loc) return;
		if (loc.is_system) {
			toast.error('No se pueden eliminar ubicaciones del sistema');
			return;
		}
		if (!confirm(`¿Eliminar la ubicación "${loc.name}"? Esto también eliminará todas las áreas asociadas.`)) return;
		await app.deleteLocation(id);
	}

	// Area actions
	function openNewArea() {
		editArea = null;
		areaModalOpen = true;
	}
	function openEditArea(id: number) {
		editArea = app.areas.find((a) => a.id === id) || null;
		areaModalOpen = true;
	}
	async function confirmDeleteArea(id: number) {
		const area = app.areas.find((a) => a.id === id);
		if (!area) return;
		if (!confirm(`¿Eliminar el área "${area.name}"?`)) return;
		await app.deleteArea(id);
	}

	// Mosaic actions
	function openNewMosaic() {
		editMosaic = null;
		mosaicModalOpen = true;
	}
	function openEditMosaic(id: number) {
		editMosaic = app.mosaics.find((m) => m.id === id) || null;
		mosaicModalOpen = true;
	}
	async function confirmDeleteMosaic(id: number) {
		const m = app.mosaics.find((m) => m.id === id);
		if (!m) return;
		if (!confirm(`¿Eliminar el mosaico "${m.name}"?`)) return;
		await app.deleteMosaic(id);
	}
	function viewMosaic(name: string) {
		viewerStreamName = 'mosaic-' + name;
		viewerOpen = true;
	}

	function debouncedSearch(e: Event) {
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			app.searchQuery = (e.target as HTMLInputElement).value;
		}, 300);
	}

	function toggleFilterLocation(name: string) {
		if (app.filterLocations.includes(name)) {
			app.filterLocations = app.filterLocations.filter((l) => l !== name);
		} else {
			app.filterLocations = [...app.filterLocations, name];
		}
	}
	function toggleFilterArea(name: string) {
		if (app.filterAreas.includes(name)) {
			app.filterAreas = app.filterAreas.filter((a) => a !== name);
		} else {
			app.filterAreas = [...app.filterAreas, name];
		}
	}

	function openNewAreaForLocation(locId: number) {
		editArea = { id: 0, name: '', location_id: locId, location_name: '', description: '', created_at: null } as AreaWithLocation;
		areaModalOpen = true;
	}

	function logout() {
		auth.logout();
	}
</script>

{#if auth.isAuthenticated}
<!-- Header -->
<header class="bg-gray-900 border-b border-gray-800 px-6 py-4 flex items-center justify-between sticky top-0 z-40">
	<div class="flex items-center gap-3">
		<div class="w-9 h-9 bg-blue-600 rounded-lg flex items-center justify-center">
			<Video class="w-5 h-5 text-white" />
		</div>
		<div>
			<h1 class="text-lg font-bold text-white">MediaMTX Camera Manager</h1>
			<p class="text-xs text-gray-500">{app.cameras.length} cámaras registradas</p>
		</div>
	</div>
	<div class="flex items-center gap-3">
		<button onclick={() => app.syncCameras()} class="flex items-center gap-2 px-3 py-2 bg-gray-800 hover:bg-gray-700 rounded-lg text-sm transition" title="Sincronizar cámaras con MediaMTX">
			<RefreshCw class="w-4 h-4" /> Sync
		</button>
		<a href="/docs" target="_blank" class="flex items-center gap-2 px-3 py-2 bg-gray-800 hover:bg-gray-700 rounded-lg text-sm transition">
			<BookOpen class="w-4 h-4" /> API Docs
		</a>
		<button onclick={logout} class="flex items-center gap-2 px-3 py-2 bg-red-900/40 hover:bg-red-800/60 text-red-300 rounded-lg text-sm transition" title="Cerrar sesión">
			<LogOut class="w-4 h-4" />
		</button>
	</div>
</header>

<!-- Tabs -->
<div class="bg-gray-900 border-b border-gray-800 px-6 flex gap-6 overflow-x-auto">
	{#each [
		{ id: 'cameras' as Tab, label: 'Cámaras', icon: Video },
		{ id: 'mosaics' as Tab, label: 'Mosaicos', icon: Grid3x3 },
		{ id: 'locations' as Tab, label: 'Ubicaciones y Áreas', icon: MapPin }
	] as tab}
		{@const Icon = tab.icon}
		<button
			onclick={() => switchTab(tab.id)}
			class="py-3 px-1 text-sm transition whitespace-nowrap {app.activeTab === tab.id
				? 'border-b-2 border-blue-500 text-blue-500 font-semibold'
				: 'text-gray-400 hover:text-gray-200'}"
		>
			<Icon class="w-4 h-4 inline mr-2" />
			{tab.label}
		</button>
	{/each}
</div>

<!-- Main Content -->
<main class="max-w-7xl mx-auto px-6 py-6">

	<!-- ═══════════════ CAMERAS TAB ═══════════════ -->
	{#if app.activeTab === 'cameras'}
		<!-- Filters -->
		<div class="bg-gray-800/50 border border-gray-700 rounded-lg p-4 mb-6">
			<div class="flex items-center justify-between mb-3">
				<h3 class="text-sm font-medium text-gray-300">Filtros</h3>
				{#if app.filterLocations.length || app.filterAreas.length || !app.filterEnabled || !app.filterRecording}
					<button onclick={() => app.clearFilters()} class="text-xs text-blue-400 hover:text-blue-300 transition">Limpiar filtros</button>
				{/if}
			</div>
			<div class="space-y-3">
				<div>
					<span class="block text-xs text-gray-500 mb-1.5">Ubicaciones</span>
					<div class="flex flex-wrap gap-1.5">
						{#each app.locations as loc}
							{@const active = app.filterLocations.includes(loc.name)}
							<button
								onclick={() => toggleFilterLocation(loc.name)}
								class="px-2.5 py-1 rounded-full text-xs transition border {active
									? 'bg-blue-600/30 border-blue-500/50 text-blue-300'
									: 'bg-gray-800 border-gray-700 text-gray-400 hover:border-gray-500'}"
							>
								<MapPin class="w-3 h-3 inline mr-1" />{loc.name}
							</button>
						{:else}
							<span class="text-xs text-gray-600">Sin ubicaciones</span>
						{/each}
					</div>
				</div>
				<div>
					<span class="block text-xs text-gray-500 mb-1.5">Áreas</span>
					<div class="flex flex-wrap gap-1.5">
						{#each app.areas as area}
							{@const active = app.filterAreas.includes(area.name)}
							<button
								onclick={() => toggleFilterArea(area.name)}
								class="px-2.5 py-1 rounded-full text-xs transition border {active
									? 'bg-teal-600/30 border-teal-500/50 text-teal-300'
									: 'bg-gray-800 border-gray-700 text-gray-400 hover:border-gray-500'}"
							>
								<Layers class="w-3 h-3 inline mr-1" />{area.name}
							</button>
						{:else}
							<span class="text-xs text-gray-600">Sin áreas</span>
						{/each}
					</div>
				</div>
				<div class="flex items-center gap-4">
					<span class="text-xs text-gray-500">Estado:</span>
					<label class="flex items-center gap-1.5 text-xs cursor-pointer">
						<input type="checkbox" bind:checked={app.filterEnabled} class="rounded bg-gray-600 border-gray-500 text-blue-600 focus:ring-blue-500 w-3.5 h-3.5" />
						Habilitadas
					</label>
					<label class="flex items-center gap-1.5 text-xs cursor-pointer">
						<input type="checkbox" bind:checked={app.filterRecording} class="rounded bg-gray-600 border-gray-500 text-blue-600 focus:ring-blue-500 w-3.5 h-3.5" />
						Grabando
					</label>
				</div>
			</div>
		</div>

		<!-- Search + Add -->
		<div class="flex items-center justify-between mb-6 gap-4">
			<div class="relative flex-1 max-w-md">
				<Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" />
				<input
					type="text"
					placeholder="Buscar cámaras por nombre, IP, ubicación o área..."
					class="w-full bg-gray-800 border border-gray-700 rounded-lg pl-10 pr-4 py-2.5 text-sm focus:outline-none focus:border-blue-500 transition"
					oninput={debouncedSearch}
				/>
			</div>
			<button onclick={openNewCamera} class="flex items-center gap-2 px-4 py-2.5 bg-blue-600 hover:bg-blue-700 rounded-lg text-sm font-medium transition">
				<Plus class="w-4 h-4" /> Nueva Cámara
			</button>
		</div>

		<!-- Camera Grid -->
		{#if app.filteredCameras.length > 0}
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
				{#each app.filteredCameras as camera (camera.id)}
					<CameraCard {camera} onEdit={openEditCamera} onDelete={confirmDeleteCamera} onView={openViewCamera} />
				{/each}
			</div>
		{:else}
			<div class="text-center py-16 text-gray-500">
				<CameraOff class="w-12 h-12 mx-auto mb-3 opacity-50" />
				<p>No se encontraron cámaras</p>
			</div>
		{/if}
	{/if}

	<!-- ═══════════════ MOSAICS TAB ═══════════════ -->
	{#if app.activeTab === 'mosaics'}
		<div class="flex items-center justify-between mb-6">
			<h2 class="text-lg font-semibold">Mosaicos</h2>
			<button onclick={openNewMosaic} class="flex items-center gap-2 px-4 py-2.5 bg-blue-600 hover:bg-blue-700 rounded-lg text-sm font-medium transition">
				<Plus class="w-4 h-4" /> Nuevo Mosaico
			</button>
		</div>
		{#if app.mosaics.length > 0}
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
				{#each app.mosaics as m (m.id)}
					<div class="bg-gray-900 border border-gray-800 rounded-xl p-5 transition hover:-translate-y-0.5 hover:shadow-lg hover:shadow-black/15">
						<div class="flex items-start justify-between mb-3">
							<div>
								<h4 class="font-medium text-white">{m.name}</h4>
								<p class="text-xs text-gray-500">Layout: {m.layout} &middot; {m.cameras.length} cámaras</p>
							</div>
							<div class="flex items-center gap-1">
								<span class="status-dot {m.active ? 'on' : 'off'}"></span>
								<span class="text-xs {m.active ? 'text-green-400' : 'text-gray-500'}">{m.active ? 'Activo' : 'Detenido'}</span>
							</div>
						</div>
						<div class="flex flex-wrap gap-1 mb-4">
							{#each m.cameras as c}
								<span class="text-[10px] px-2 py-0.5 bg-gray-800 text-gray-300 rounded-full">{c.camera_name}</span>
							{/each}
						</div>
						<div class="flex gap-2">
							{#if m.active}
								<button onclick={() => app.stopMosaic(m.id)} class="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded-lg text-sm transition">
									<Square class="w-3.5 h-3.5" /> Detener
								</button>
								<button onclick={() => viewMosaic(m.name)} class="flex items-center justify-center gap-2 px-3 py-2 bg-blue-600/20 hover:bg-blue-600/30 text-blue-400 rounded-lg text-sm transition">
									<Eye class="w-3.5 h-3.5" /> Ver
								</button>
							{:else}
								<button onclick={() => app.startMosaic(m.id)} class="flex-1 flex items-center justify-center gap-2 px-3 py-2 bg-green-600/20 hover:bg-green-600/30 text-green-400 rounded-lg text-sm transition">
									<Play class="w-3.5 h-3.5" /> Iniciar
								</button>
							{/if}
							<button onclick={() => openEditMosaic(m.id)} class="px-3 py-2 bg-gray-800 hover:bg-gray-700 rounded-lg text-sm transition">
								<Pencil class="w-3.5 h-3.5" />
							</button>
							<button onclick={() => confirmDeleteMosaic(m.id)} class="px-3 py-2 bg-gray-800 hover:bg-gray-700 text-red-400 rounded-lg text-sm transition">
								<Trash2 class="w-3.5 h-3.5" />
							</button>
						</div>
						{#if m.active}
							<div class="mt-3 flex flex-wrap gap-2 text-xs text-gray-500">
								<span class="flex items-center gap-1">
									<Tv class="w-3 h-3" />
									<a href="{globalThis.location?.protocol}//{globalThis.location?.hostname}:8889/mosaic-{m.name}" target="_blank" class="text-blue-400 hover:underline">WebRTC</a>
								</span>
								<span class="flex items-center gap-1">
									<Radio class="w-3 h-3" />
									<a href="{globalThis.location?.protocol}//{globalThis.location?.hostname}:8888/mosaic-{m.name}/index.m3u8" target="_blank" class="text-blue-400 hover:underline">HLS</a>
								</span>
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{:else}
			<div class="text-center py-16 text-gray-500">
				<LayoutGrid class="w-12 h-12 mx-auto mb-3 opacity-50" />
				<p>No hay mosaicos creados</p>
			</div>
		{/if}
	{/if}

	<!-- ═══════════════ LOCATIONS & AREAS TAB ═══════════════ -->
	{#if app.activeTab === 'locations'}
		<div class="flex items-center justify-between mb-6">
			<h2 class="text-lg font-semibold">Ubicaciones y Áreas</h2>
			<div class="flex gap-2">
				<button onclick={openNewArea} class="flex items-center gap-2 px-4 py-2.5 bg-gray-800 hover:bg-gray-700 rounded-lg text-sm font-medium transition border border-gray-700">
					<Plus class="w-4 h-4" /> Nueva Área
				</button>
				<button onclick={openNewLocation} class="flex items-center gap-2 px-4 py-2.5 bg-blue-600 hover:bg-blue-700 rounded-lg text-sm font-medium transition">
					<Plus class="w-4 h-4" /> Nueva Ubicación
				</button>
			</div>
		</div>
		{#if app.locations.length > 0}
			<div class="space-y-4">
				{#each app.locations as loc (loc.id)}
					{@const locAreas = app.areas.filter((a) => a.location_id === loc.id)}
					{@const locCameras = app.cameras.filter((c) => c.location === loc.name)}
					<div class="bg-gray-800 border border-gray-700 rounded-xl overflow-hidden hover:border-gray-600 transition">
						<!-- Location Header -->
						<div class="px-5 py-4 flex items-center justify-between">
							<div class="flex items-center gap-3">
								<div class="w-10 h-10 bg-blue-600/20 rounded-lg flex items-center justify-center shrink-0">
									<MapPin class="w-5 h-5 text-blue-400" />
								</div>
								<div>
									<div class="flex items-center gap-2">
										<h4 class="font-semibold text-white">{loc.name}</h4>
										{#if loc.is_system}
											<span class="px-2 py-0.5 bg-purple-600/20 text-purple-400 text-[10px] rounded-full border border-purple-600/30">Sistema</span>
										{/if}
									</div>
									{#if loc.description}
										<p class="text-xs text-gray-500 mt-0.5">{loc.description}</p>
									{/if}
								</div>
							</div>
							<div class="flex items-center gap-3">
								<div class="flex items-center gap-3 text-xs text-gray-500 mr-2">
									<span class="inline-flex items-center gap-1">
										<Layers class="w-3 h-3" /> {locAreas.length} áreas
									</span>
									<span class="inline-flex items-center gap-1">
										<Video class="w-3 h-3" /> {locCameras.length} cámaras
									</span>
								</div>
								<button onclick={() => openEditLocation(loc.id)} class="p-1.5 text-gray-400 hover:text-amber-400 transition rounded hover:bg-gray-700" title="Editar ubicación">
									<Edit2 class="w-4 h-4" />
								</button>
								{#if !loc.is_system}
									<button onclick={() => confirmDeleteLocation(loc.id)} class="p-1.5 text-gray-400 hover:text-red-400 transition rounded hover:bg-gray-700" title="Eliminar ubicación">
										<Trash2 class="w-4 h-4" />
									</button>
								{:else}
									<button disabled class="p-1.5 text-gray-600 cursor-not-allowed rounded" title="No se puede eliminar">
										<Lock class="w-4 h-4" />
									</button>
								{/if}
							</div>
						</div>
						<!-- Areas List -->
						<div class="border-t border-gray-700/50 bg-gray-850">
							{#if locAreas.length > 0}
								<div class="divide-y divide-gray-700/30">
									{#each locAreas as area (area.id)}
										{@const areaCameras = app.cameras.filter((c) => c.area === area.name && c.location === loc.name)}
										<div class="px-5 py-3 flex items-center justify-between hover:bg-gray-700/20 transition group">
											<div class="flex items-center gap-3">
												<div class="w-7 h-7 bg-teal-600/20 rounded-md flex items-center justify-center shrink-0">
													<Layers class="w-3.5 h-3.5 text-teal-400" />
												</div>
												<div>
													<span class="text-sm text-white">{area.name}</span>
													{#if area.description}
														<p class="text-xs text-gray-500">{area.description}</p>
													{/if}
												</div>
											</div>
											<div class="flex items-center gap-2">
												<span class="text-xs text-gray-500 inline-flex items-center gap-1">
													<Video class="w-3 h-3" /> {areaCameras.length}
												</span>
												<button onclick={() => openEditArea(area.id)} class="p-1 text-gray-500 hover:text-amber-400 transition rounded opacity-0 group-hover:opacity-100" title="Editar área">
													<Edit2 class="w-3.5 h-3.5" />
												</button>
												<button onclick={() => confirmDeleteArea(area.id)} class="p-1 text-gray-500 hover:text-red-400 transition rounded opacity-0 group-hover:opacity-100" title="Eliminar área">
													<Trash2 class="w-3.5 h-3.5" />
												</button>
											</div>
										</div>
									{/each}
								</div>
							{:else}
								<div class="px-5 py-3 text-xs text-gray-600 italic">Sin áreas definidas</div>
							{/if}
							<div class="px-5 py-2 border-t border-gray-700/30">
								<button onclick={() => openNewAreaForLocation(loc.id)} class="text-xs text-blue-400 hover:text-blue-300 transition flex items-center gap-1">
									<Plus class="w-3 h-3" /> Agregar área a {loc.name}
								</button>
							</div>
						</div>
					</div>
				{/each}
			</div>
		{:else}
			<div class="text-center py-16 text-gray-500">
				<MapPinOff class="w-12 h-12 mx-auto mb-3 opacity-50" />
				<p>No hay ubicaciones creadas</p>
			</div>
		{/if}
	{/if}
</main>

<!-- Modals -->
<CameraModal bind:open={cameraModalOpen} {editCamera} />
<LocationModal bind:open={locationModalOpen} {editLocation} />
<AreaModal bind:open={areaModalOpen} {editArea} />
<MosaicModal bind:open={mosaicModalOpen} {editMosaic} />
<CameraViewer bind:open={viewerOpen} streamName={viewerStreamName} />
{/if}
