<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { app } from '$lib/stores/app.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import type { Camera, Location, AreaWithLocation, Mosaic, UserPublic, RoleWithPermissions } from '$lib/types';
	import ShareMosaicModal from '$lib/components/ShareMosaicModal.svelte';
	import CameraCard from '$lib/components/CameraCard.svelte';
	import CameraModal from '$lib/components/CameraModal.svelte';
	import CameraViewer from '$lib/components/CameraViewer.svelte';
	import LocationModal from '$lib/components/LocationModal.svelte';
	import AreaModal from '$lib/components/AreaModal.svelte';
	import MosaicModal from '$lib/components/MosaicModal.svelte';
	import UserModal from '$lib/components/UserModal.svelte';
	import RoleModal from '$lib/components/RoleModal.svelte';
	import ConfirmDeleteDialog from '$lib/components/ConfirmDeleteDialog.svelte';
	import NotificationPanel from '$lib/components/NotificationPanel.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import {
		Video, Layers, Plus, Search, CameraOff, LayoutGrid, MapPin, MapPinOff, Edit2, Trash2,
		Lock, Eye, Pencil, Play, Square, Tv, Radio, Menu, Users, Shield, UserCheck, UserX,
		Image, Calendar, Download, Bell, BellOff, CheckCheck, ToggleLeft, ToggleRight, Share2
	} from 'lucide-svelte';

	let cameraModalOpen = $state(false);
	let editCamera = $state<Camera | null>(null);
	let locationModalOpen = $state(false);
	let editLocation = $state<Location | null>(null);
	let areaModalOpen = $state(false);
	let editArea = $state<AreaWithLocation | null>(null);
	let mosaicModalOpen = $state(false);
	let editMosaic = $state<Mosaic | null>(null);
	let userModalOpen = $state(false);
	let editUser = $state<UserPublic | null>(null);
	let roleModalOpen = $state(false);
	let editRole = $state<RoleWithPermissions | null>(null);
	let shareModalOpen = $state(false);
	let shareMosaic = $state<Mosaic | null>(null);
	let viewerOpen = $state(false);
	let viewerStreamName = $state('');
	let sidebarCollapsed = $state(false);
	let sidebarMobileOpen = $state(false);
	let filtersOpen = $state(false);
	let searchTimeout: ReturnType<typeof setTimeout>;

	// Delete dialog state
	let deleteDialogOpen = $state(false);
	let deleteDialogTitle = $state('');
	let deleteDialogMessage = $state('');
	let deleteDialogAction = $state<() => void>(() => {});

	// Captures filters
	let captureFilterCamera = $state(0);
	let captureFilterDate = $state('');

	// Notification panel
	let notifPanelOpen = $state(false);

	onMount(() => {
		if (!auth.isAuthenticated) { goto('/login'); return; }
		app.loadAll();
	});
	onDestroy(() => { app.stopAll(); });

	// Generic delete dialog
	function openDeleteDialog(title: string, message: string, action: () => void) {
		deleteDialogTitle = title;
		deleteDialogMessage = message;
		deleteDialogAction = action;
		deleteDialogOpen = true;
	}

	// Camera actions
	function openNewCamera() { editCamera = null; cameraModalOpen = true; }
	function openEditCamera(id: number) { editCamera = app.cameras.find((c) => c.id === id) || null; cameraModalOpen = true; }
	function confirmDeleteCamera(id: number) {
		const cam = app.cameras.find((c) => c.id === id);
		if (!cam) return;
		openDeleteDialog('Eliminar Cámara', `¿Eliminar la cámara "${cam.name}"? Esta acción no se puede deshacer.`, () => app.deleteCamera(id));
	}
	function openViewCamera(id: number) {
		const cam = app.cameras.find((c) => c.id === id);
		if (!cam) return;
		viewerStreamName = cam.name; viewerOpen = true;
	}

	// Location actions
	function openNewLocation() { editLocation = null; locationModalOpen = true; }
	function openEditLocation(id: number) { editLocation = app.locations.find((l) => l.id === id) || null; locationModalOpen = true; }
	function confirmDeleteLocation(id: number) {
		const loc = app.locations.find((l) => l.id === id);
		if (!loc) return;
		if (loc.is_system) { toast.error('No se pueden eliminar ubicaciones del sistema'); return; }
		openDeleteDialog('Eliminar Ubicación', `¿Eliminar "${loc.name}"? Se eliminarán todas las áreas asociadas.`, () => app.deleteLocation(id));
	}

	// Area actions
	function openNewArea() { editArea = null; areaModalOpen = true; }
	function openEditArea(id: number) { editArea = app.areas.find((a) => a.id === id) || null; areaModalOpen = true; }
	function confirmDeleteArea(id: number) {
		const area = app.areas.find((a) => a.id === id);
		if (!area) return;
		openDeleteDialog('Eliminar Área', `¿Eliminar el área "${area.name}"?`, () => app.deleteArea(id));
	}
	function openNewAreaForLocation(locId: number) {
		editArea = { id: 0, name: '', location_id: locId, location_name: '', description: '', created_at: null } as AreaWithLocation;
		areaModalOpen = true;
	}

	// Mosaic actions
	function openNewMosaic() { editMosaic = null; mosaicModalOpen = true; }
	function openEditMosaic(id: number) { editMosaic = app.mosaics.find((m) => m.id === id) || null; mosaicModalOpen = true; }
	function confirmDeleteMosaic(id: number) {
		const m = app.mosaics.find((m) => m.id === id);
		if (!m) return;
		openDeleteDialog('Eliminar Mosaico', `¿Eliminar el mosaico "${m.name}"?`, () => app.deleteMosaic(id));
	}
	function viewMosaic(name: string) { viewerStreamName = 'mosaic-' + name; viewerOpen = true; }
	function openShareMosaic(m: Mosaic) { shareMosaic = m; shareModalOpen = true; }

	// User actions
	function openNewUser() { editUser = null; userModalOpen = true; }
	function openEditUser(u: UserPublic) { editUser = u; userModalOpen = true; }
	function confirmDeleteUser(u: UserPublic) {
		openDeleteDialog('Eliminar Usuario', `¿Eliminar el usuario "${u.username}"?`, () => app.deleteUser(u.id));
	}

	// Role actions
	function openNewRole() { editRole = null; roleModalOpen = true; }
	function openEditRole(r: RoleWithPermissions) { editRole = r; roleModalOpen = true; }
	function confirmDeleteRole(r: RoleWithPermissions) {
		if (r.is_system) { toast.error('No se pueden eliminar roles del sistema'); return; }
		openDeleteDialog('Eliminar Rol', `¿Eliminar el rol "${r.name}"?`, () => app.deleteRole(r.id));
	}

	// Captures
	function loadCapturesFiltered() {
		app.loadCaptures(captureFilterCamera || undefined, captureFilterDate || undefined);
	}

	$effect(() => {
		if (app.activeTab === 'captures') {
			loadCapturesFiltered();
		}
	});

	// Filters
	function debouncedSearch(e: Event) {
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => { app.searchQuery = (e.target as HTMLInputElement).value; }, 300);
	}
	function toggleFilterLocation(name: string) {
		app.filterLocations = app.filterLocations.includes(name)
			? app.filterLocations.filter((l) => l !== name) : [...app.filterLocations, name];
	}
	function toggleFilterArea(name: string) {
		app.filterAreas = app.filterAreas.includes(name)
			? app.filterAreas.filter((a) => a !== name) : [...app.filterAreas, name];
	}

	let hasActiveFilters = $derived(app.filterLocations.length > 0 || app.filterAreas.length > 0 || !app.filterEnabled || !app.filterRecording);

	const roleLabels: Record<string, string> = { admin: 'Administrador', operator: 'Operador', viewer: 'Visor' };

	const moduleLabels: Record<string, string> = {
		cameras: 'Cámaras', mosaics: 'Mosaicos', locations: 'Ubicaciones', users: 'Usuarios',
		captures: 'Capturas', notifications: 'Notificaciones', roles: 'Roles', settings: 'Ajustes'
	};
</script>

{#if auth.isAuthenticated}
<div class="flex h-screen overflow-hidden bg-surface">
	<!-- Sidebar -->
	<Sidebar bind:collapsed={sidebarCollapsed} bind:mobileOpen={sidebarMobileOpen} />

	<!-- Main content area -->
	<div class="flex-1 flex flex-col min-w-0 overflow-hidden">
		<!-- Top bar (mobile: hamburger + title) -->
		<header class="lg:hidden bg-surface-alt border-b border-edge px-4 py-3 flex items-center gap-3 shrink-0 safe-top" style="box-shadow: 0 1px 3px var(--th-shadow);">
			<button onclick={() => (sidebarMobileOpen = true)} class="btn btn-ghost p-2">
				<Menu class="w-5 h-5" />
			</button>
			<h1 class="text-sm font-bold text-content truncate flex-1">
				{#if app.activeTab === 'cameras'}Cámaras
				{:else if app.activeTab === 'mosaics'}Mosaicos
				{:else if app.activeTab === 'locations'}Ubicaciones
				{:else if app.activeTab === 'users'}Usuarios
				{:else if app.activeTab === 'captures'}Capturas
				{:else if app.activeTab === 'notifications'}Notificaciones
				{:else if app.activeTab === 'roles'}Roles y Permisos
				{/if}
			</h1>
			<button onclick={() => (notifPanelOpen = true)} class="btn btn-ghost p-2 relative">
				<Bell class="w-5 h-5" />
				{#if app.unreadCount > 0}
					<span class="absolute -top-0.5 -right-0.5 w-4 h-4 bg-red-500 text-white text-[9px] font-bold rounded-full flex items-center justify-center">{app.unreadCount > 9 ? '9+' : app.unreadCount}</span>
				{/if}
			</button>
		</header>

		<!-- Scrollable content -->
		<main class="flex-1 overflow-y-auto overflow-x-hidden safe-bottom">
			<div class="max-w-7xl mx-auto px-4 sm:px-6 py-4 sm:py-6">

			<!-- ═══════════════ CAMERAS ═══════════════ -->
			{#if app.activeTab === 'cameras'}
				<div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between mb-4 gap-3">
					<div class="relative flex-1 max-w-md">
						<Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-content-muted" />
						<input type="text" placeholder="Buscar cámaras..." class="input pl-10" oninput={debouncedSearch} />
					</div>
					<div class="flex gap-2">
						<button onclick={() => (filtersOpen = !filtersOpen)}
							class="btn btn-secondary py-2 text-xs {hasActiveFilters ? 'border-primary text-primary' : ''}">
							<Layers class="w-3.5 h-3.5" /> Filtros {hasActiveFilters ? '●' : ''}
						</button>
						<button onclick={openNewCamera} class="btn btn-primary py-2 text-xs">
							<Plus class="w-3.5 h-3.5" /> <span class="hidden sm:inline">Nueva Cámara</span><span class="sm:hidden">Nueva</span>
						</button>
					</div>
				</div>
				{#if filtersOpen}
					<div class="bg-surface-alt border border-edge rounded-xl p-4 mb-4 fade-in" style="box-shadow: 0 1px 3px var(--th-shadow);">
						<div class="flex items-center justify-between mb-3">
							<h3 class="text-sm font-medium text-content">Filtros</h3>
							{#if hasActiveFilters}
								<button onclick={() => app.clearFilters()} class="text-xs text-primary hover:underline">Limpiar</button>
							{/if}
						</div>
						<div class="space-y-3">
							<div>
								<span class="block text-xs text-content-muted mb-1.5">Ubicaciones</span>
								<div class="flex flex-wrap gap-1.5">
									{#each app.locations as loc}
										{@const active = app.filterLocations.includes(loc.name)}
										<button onclick={() => toggleFilterLocation(loc.name)}
											class="px-2.5 py-1 rounded-full text-xs transition border
												{active ? 'bg-primary/15 border-primary/40 text-primary' : 'bg-surface-raised border-edge text-content-muted hover:border-edge-secondary'}">
											<MapPin class="w-3 h-3 inline mr-1" />{loc.name}
										</button>
									{:else}
										<span class="text-xs text-content-muted italic">Sin ubicaciones</span>
									{/each}
								</div>
							</div>
							<div>
								<span class="block text-xs text-content-muted mb-1.5">Áreas</span>
								<div class="flex flex-wrap gap-1.5">
									{#each app.areas as area}
										{@const active = app.filterAreas.includes(area.name)}
										<button onclick={() => toggleFilterArea(area.name)}
											class="px-2.5 py-1 rounded-full text-xs transition border
												{active ? 'bg-emerald-500/15 border-emerald-500/40 text-emerald-600 dark:text-emerald-400' : 'bg-surface-raised border-edge text-content-muted hover:border-edge-secondary'}">
											<Layers class="w-3 h-3 inline mr-1" />{area.name}
										</button>
									{:else}
										<span class="text-xs text-content-muted italic">Sin áreas</span>
									{/each}
								</div>
							</div>
							<div class="flex flex-wrap items-center gap-4 pt-1">
								<span class="text-xs text-content-muted">Estado:</span>
								<label class="flex items-center gap-1.5 text-xs text-content-secondary cursor-pointer">
									<input type="checkbox" bind:checked={app.filterEnabled} class="rounded bg-surface-raised border-edge text-blue-600 focus:ring-blue-500 w-3.5 h-3.5" />
									Habilitadas
								</label>
								<label class="flex items-center gap-1.5 text-xs text-content-secondary cursor-pointer">
									<input type="checkbox" bind:checked={app.filterRecording} class="rounded bg-surface-raised border-edge text-blue-600 focus:ring-blue-500 w-3.5 h-3.5" />
									Grabando
								</label>
							</div>
						</div>
					</div>
				{/if}
				{#if app.filteredCameras.length > 0}
					<p class="text-xs text-content-muted mb-3">{app.filteredCameras.length} de {app.cameras.length} cámaras</p>
					<div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4 gap-3 sm:gap-4">
						{#each app.filteredCameras as camera (camera.id)}
							<CameraCard {camera} onEdit={openEditCamera} onDelete={confirmDeleteCamera} onView={openViewCamera} onScreenshot={(id) => app.takeScreenshot(id)} />
						{/each}
					</div>
				{:else}
					<div class="text-center py-16 text-content-muted">
						<CameraOff class="w-12 h-12 mx-auto mb-3 opacity-40" />
						<p class="text-sm">No se encontraron cámaras</p>
					</div>
				{/if}
			{/if}

			<!-- ═══════════════ MOSAICS ═══════════════ -->
			{#if app.activeTab === 'mosaics'}
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-lg font-semibold text-content">Mosaicos</h2>
					<button onclick={openNewMosaic} class="btn btn-primary py-2 text-xs">
						<Plus class="w-3.5 h-3.5" /> Nuevo Mosaico
					</button>
				</div>
				{#if app.mosaics.length > 0}
					<div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-3 sm:gap-4">
						{#each app.mosaics as m (m.id)}
							<div class="bg-surface-alt border border-edge rounded-xl p-4 sm:p-5 transition hover:-translate-y-0.5 hover:shadow-lg" style="box-shadow: 0 1px 3px var(--th-shadow);">
								<div class="flex items-start justify-between mb-3">
									<div class="min-w-0">
										<h4 class="font-medium text-content truncate">{m.name}</h4>
										<p class="text-xs text-content-muted">Layout: {m.layout} &middot; {m.cameras.length} cámaras</p>
									</div>
									<div class="flex items-center gap-1.5 shrink-0">
										<span class="status-dot {m.active ? 'on' : 'off'}"></span>
										<span class="text-xs {m.active ? 'text-success' : 'text-content-muted'}">{m.active ? 'Activo' : 'Detenido'}</span>
									</div>
								</div>
								<div class="flex flex-wrap gap-1 mb-4">
									{#each m.cameras as c}
										<span class="badge badge-neutral">{c.camera_name}</span>
									{/each}
								</div>
								<div class="flex gap-2">
									{#if m.active}
										<button onclick={() => app.stopMosaic(m.id)} class="flex-1 btn btn-danger py-1.5 text-xs">
											<Square class="w-3.5 h-3.5" /> Detener
										</button>
										<button onclick={() => viewMosaic(m.name)} class="btn btn-secondary py-1.5 text-xs">
											<Eye class="w-3.5 h-3.5" /> Ver
										</button>
									{:else}
										<button onclick={() => app.startMosaic(m.id)} class="flex-1 btn py-1.5 text-xs" style="background:var(--th-badge-success-bg);color:var(--th-badge-success-text);">
											<Play class="w-3.5 h-3.5" /> Iniciar
										</button>
									{/if}
									<button onclick={() => openShareMosaic(m)} class="btn btn-ghost p-1.5" title="Compartir">
										<Share2 class="w-3.5 h-3.5" />
									</button>
									<button onclick={() => openEditMosaic(m.id)} class="btn btn-ghost p-1.5">
										<Pencil class="w-3.5 h-3.5" />
									</button>
									<button onclick={() => confirmDeleteMosaic(m.id)} class="btn btn-ghost p-1.5 text-destructive">
										<Trash2 class="w-3.5 h-3.5" />
									</button>
								</div>
								{#if m.active}
									<div class="mt-3 flex flex-wrap gap-3 text-xs text-content-muted">
										<span class="flex items-center gap-1"><Tv class="w-3 h-3" />
											<a href="{globalThis.location?.protocol}//{globalThis.location?.hostname}:8889/mosaic-{m.name}" target="_blank" class="text-primary hover:underline">WebRTC</a>
										</span>
										<span class="flex items-center gap-1"><Radio class="w-3 h-3" />
											<a href="{globalThis.location?.protocol}//{globalThis.location?.hostname}:8888/mosaic-{m.name}/index.m3u8" target="_blank" class="text-primary hover:underline">HLS</a>
										</span>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-16 text-content-muted">
						<LayoutGrid class="w-12 h-12 mx-auto mb-3 opacity-40" />
						<p class="text-sm">No hay mosaicos creados</p>
					</div>
				{/if}
			{/if}

			<!-- ═══════════════ LOCATIONS & AREAS ═══════════════ -->
			{#if app.activeTab === 'locations'}
				<div class="flex flex-col sm:flex-row items-start sm:items-center justify-between mb-4 gap-3">
					<h2 class="text-lg font-semibold text-content">Ubicaciones y Áreas</h2>
					<div class="flex gap-2">
						<button onclick={openNewArea} class="btn btn-secondary py-2 text-xs">
							<Plus class="w-3.5 h-3.5" /> Nueva Área
						</button>
						<button onclick={openNewLocation} class="btn btn-primary py-2 text-xs">
							<Plus class="w-3.5 h-3.5" /> Nueva Ubicación
						</button>
					</div>
				</div>
				{#if app.locations.length > 0}
					<div class="space-y-3">
						{#each app.locations as loc (loc.id)}
							{@const locAreas = app.areas.filter((a) => a.location_id === loc.id)}
							{@const locCameras = app.cameras.filter((c) => c.location === loc.name)}
							<div class="bg-surface-alt border border-edge rounded-xl overflow-hidden transition hover:border-edge-secondary" style="box-shadow: 0 1px 3px var(--th-shadow);">
								<div class="px-4 sm:px-5 py-4 flex items-center justify-between">
									<div class="flex items-center gap-3 min-w-0">
										<div class="w-10 h-10 rounded-lg flex items-center justify-center shrink-0" style="background:var(--th-badge-info-bg);">
											<MapPin class="w-5 h-5" style="color:var(--th-badge-info-text);" />
										</div>
										<div class="min-w-0">
											<div class="flex items-center gap-2 flex-wrap">
												<h4 class="font-semibold text-content truncate">{loc.name}</h4>
												{#if loc.is_system}<span class="badge badge-warn">Sistema</span>{/if}
											</div>
											{#if loc.description}<p class="text-xs text-content-muted mt-0.5 truncate">{loc.description}</p>{/if}
										</div>
									</div>
									<div class="flex items-center gap-2 shrink-0">
										<div class="hidden sm:flex items-center gap-3 text-xs text-content-muted mr-2">
											<span class="inline-flex items-center gap-1"><Layers class="w-3 h-3" /> {locAreas.length}</span>
											<span class="inline-flex items-center gap-1"><Video class="w-3 h-3" /> {locCameras.length}</span>
										</div>
										<button onclick={() => openEditLocation(loc.id)} class="btn btn-ghost p-1.5" title="Editar">
											<Edit2 class="w-4 h-4" />
										</button>
										{#if !loc.is_system}
											<button onclick={() => confirmDeleteLocation(loc.id)} class="btn btn-ghost p-1.5 text-destructive" title="Eliminar">
												<Trash2 class="w-4 h-4" />
											</button>
										{:else}
											<button disabled class="p-1.5 text-content-muted/30 cursor-not-allowed rounded" title="No se puede eliminar">
												<Lock class="w-4 h-4" />
											</button>
										{/if}
									</div>
								</div>
								<div class="border-t border-edge/50 bg-surface-inset">
									{#if locAreas.length > 0}
										<div class="divide-y divide-edge/30">
											{#each locAreas as area (area.id)}
												{@const areaCameras = app.cameras.filter((c) => c.area === area.name && c.location === loc.name)}
												<div class="px-4 sm:px-5 py-2.5 flex items-center justify-between hover:bg-surface-hover/30 transition group">
													<div class="flex items-center gap-3 min-w-0">
														<div class="w-7 h-7 rounded-md flex items-center justify-center shrink-0" style="background:var(--th-badge-success-bg);">
															<Layers class="w-3.5 h-3.5" style="color:var(--th-badge-success-text);" />
														</div>
														<div class="min-w-0">
															<span class="text-sm text-content">{area.name}</span>
															{#if area.description}<p class="text-xs text-content-muted truncate">{area.description}</p>{/if}
														</div>
													</div>
													<div class="flex items-center gap-2 shrink-0">
														<span class="text-xs text-content-muted inline-flex items-center gap-1"><Video class="w-3 h-3" /> {areaCameras.length}</span>
														<button onclick={() => openEditArea(area.id)} class="p-1 text-content-muted hover:text-amber-500 transition rounded opacity-0 group-hover:opacity-100" title="Editar">
															<Edit2 class="w-3.5 h-3.5" />
														</button>
														<button onclick={() => confirmDeleteArea(area.id)} class="p-1 text-content-muted hover:text-destructive transition rounded opacity-0 group-hover:opacity-100" title="Eliminar">
															<Trash2 class="w-3.5 h-3.5" />
														</button>
													</div>
												</div>
											{/each}
										</div>
									{:else}
										<div class="px-5 py-3 text-xs text-content-muted italic">Sin áreas definidas</div>
									{/if}
									<div class="px-5 py-2 border-t border-edge/30">
										<button onclick={() => openNewAreaForLocation(loc.id)} class="text-xs text-primary hover:underline transition flex items-center gap-1">
											<Plus class="w-3 h-3" /> Agregar área
										</button>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-16 text-content-muted">
						<MapPinOff class="w-12 h-12 mx-auto mb-3 opacity-40" />
						<p class="text-sm">No hay ubicaciones creadas</p>
					</div>
				{/if}
			{/if}

			<!-- ═══════════════ USERS ═══════════════ -->
			{#if app.activeTab === 'users'}
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-lg font-semibold text-content">Gestión de Usuarios</h2>
					<button onclick={openNewUser} class="btn btn-primary py-2 text-xs">
						<Plus class="w-3.5 h-3.5" /> Nuevo Usuario
					</button>
				</div>
				{#if app.users.length > 0}
					<div class="bg-surface-alt border border-edge rounded-xl overflow-hidden" style="box-shadow: 0 1px 3px var(--th-shadow);">
						<!-- Desktop table -->
						<div class="hidden sm:block overflow-x-auto">
							<table class="w-full text-sm">
								<thead>
									<tr class="border-b border-edge text-content-muted text-xs text-left">
										<th class="px-4 py-3 font-medium">Usuario</th>
										<th class="px-4 py-3 font-medium">Email</th>
										<th class="px-4 py-3 font-medium">Rol</th>
										<th class="px-4 py-3 font-medium">Estado</th>
										<th class="px-4 py-3 font-medium text-right">Acciones</th>
									</tr>
								</thead>
								<tbody class="divide-y divide-edge/50">
									{#each app.users as u (u.id)}
										<tr class="hover:bg-surface-hover/30 transition">
											<td class="px-4 py-3">
												<div class="flex items-center gap-2">
													<div class="w-8 h-8 rounded-full flex items-center justify-center shrink-0"
														style="background:var(--th-badge-info-bg); color:var(--th-badge-info-text);">
														<span class="text-xs font-bold uppercase">{u.username.charAt(0)}</span>
													</div>
													<span class="text-content font-medium">{u.username}</span>
												</div>
											</td>
											<td class="px-4 py-3 text-content-secondary">{u.email}</td>
											<td class="px-4 py-3">
												<span class="badge {u.role === 'admin' ? 'badge-warn' : u.role === 'operator' ? 'badge-info' : 'badge-neutral'}">
													{roleLabels[u.role] || u.role}
												</span>
											</td>
											<td class="px-4 py-3">
												{#if u.active}
													<span class="inline-flex items-center gap-1 text-xs text-success"><UserCheck class="w-3 h-3" /> Activo</span>
												{:else}
													<span class="inline-flex items-center gap-1 text-xs text-destructive"><UserX class="w-3 h-3" /> Inactivo</span>
												{/if}
											</td>
											<td class="px-4 py-3 text-right">
												<div class="flex items-center justify-end gap-1">
													<button onclick={() => openEditUser(u)} class="btn btn-ghost p-1.5" title="Editar">
														<Edit2 class="w-3.5 h-3.5" />
													</button>
													<button onclick={() => confirmDeleteUser(u)} class="btn btn-ghost p-1.5 text-destructive" title="Eliminar">
														<Trash2 class="w-3.5 h-3.5" />
													</button>
												</div>
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
						<!-- Mobile cards -->
						<div class="sm:hidden divide-y divide-edge/50">
							{#each app.users as u (u.id)}
								<div class="p-4 flex items-center justify-between">
									<div class="flex items-center gap-3 min-w-0">
										<div class="w-10 h-10 rounded-full flex items-center justify-center shrink-0"
											style="background:var(--th-badge-info-bg); color:var(--th-badge-info-text);">
											<span class="text-sm font-bold uppercase">{u.username.charAt(0)}</span>
										</div>
										<div class="min-w-0">
											<p class="text-sm font-medium text-content truncate">{u.username}</p>
											<p class="text-xs text-content-muted truncate">{u.email}</p>
											<div class="flex items-center gap-2 mt-1">
												<span class="badge text-[10px] {u.role === 'admin' ? 'badge-warn' : 'badge-neutral'}">{roleLabels[u.role] || u.role}</span>
												{#if !u.active}<span class="text-[10px] text-destructive">Inactivo</span>{/if}
											</div>
										</div>
									</div>
									<div class="flex gap-1 shrink-0">
										<button onclick={() => openEditUser(u)} class="btn btn-ghost p-2"><Edit2 class="w-4 h-4" /></button>
										<button onclick={() => confirmDeleteUser(u)} class="btn btn-ghost p-2 text-destructive"><Trash2 class="w-4 h-4" /></button>
									</div>
								</div>
							{/each}
						</div>
					</div>
				{:else}
					<div class="text-center py-16 text-content-muted">
						<Users class="w-12 h-12 mx-auto mb-3 opacity-40" />
						<p class="text-sm">No hay usuarios registrados</p>
					</div>
				{/if}
			{/if}

			<!-- ═══════════════ CAPTURES ═══════════════ -->
			{#if app.activeTab === 'captures'}
				<div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between mb-4 gap-3">
					<h2 class="text-lg font-semibold text-content">Capturas y Screenshots</h2>
					<div class="flex items-center gap-2">
						<button onclick={() => app.toggleThumbnails()}
							class="btn btn-secondary py-2 text-xs flex items-center gap-1.5">
							{#if app.thumbnailsEnabled}<ToggleRight class="w-4 h-4 text-emerald-500" />{:else}<ToggleLeft class="w-4 h-4" />{/if}
							Miniaturas
						</button>
					</div>
				</div>
				<div class="flex flex-col sm:flex-row gap-3 mb-4">
					<select bind:value={captureFilterCamera} onchange={loadCapturesFiltered} class="input max-w-xs">
						<option value={0}>Todas las cámaras</option>
						{#each app.cameras as cam}
							<option value={cam.id}>{cam.name}</option>
						{/each}
					</select>
					<input type="date" bind:value={captureFilterDate} onchange={loadCapturesFiltered} class="input max-w-xs" />
					{#if captureFilterCamera || captureFilterDate}
						<button onclick={() => { captureFilterCamera = 0; captureFilterDate = ''; loadCapturesFiltered(); }}
							class="btn btn-secondary py-2 text-xs">Limpiar</button>
					{/if}
				</div>
				{#if app.captures.length > 0}
					<div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3">
						{#each app.captures as cap (cap.id)}
							<div class="bg-surface-alt border border-edge rounded-xl overflow-hidden group transition hover:-translate-y-0.5 hover:shadow-lg" style="box-shadow: 0 1px 3px var(--th-shadow);">
								{#if cap.capture_type === 'screenshot'}
									<div class="aspect-video bg-surface-raised flex items-center justify-center relative overflow-hidden">
										<img src="/data/captures/{cap.file_path}" alt={cap.camera_name}
											class="w-full h-full object-cover"
											onerror={(e) => { (e.target as HTMLImageElement).style.display='none'; }} />
										<Image class="w-8 h-8 text-content-muted/30 absolute" />
									</div>
								{:else}
									<div class="aspect-video bg-surface-raised flex items-center justify-center">
										<Video class="w-8 h-8 text-content-muted/30" />
									</div>
								{/if}
								<div class="p-2.5">
									<p class="text-xs font-medium text-content truncate">{cap.camera_name}</p>
									<p class="text-[10px] text-content-muted mt-0.5">
										{cap.created_at ? new Date(cap.created_at + 'Z').toLocaleString() : ''}
									</p>
									<div class="flex items-center justify-between mt-1.5">
										<span class="badge badge-neutral text-[9px]">{cap.capture_type}</span>
										<button onclick={() => app.deleteCapture(cap.id)}
											class="p-1 text-content-muted hover:text-destructive transition rounded opacity-0 group-hover:opacity-100">
											<Trash2 class="w-3 h-3" />
										</button>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-16 text-content-muted">
						<Image class="w-12 h-12 mx-auto mb-3 opacity-40" />
						<p class="text-sm">No hay capturas</p>
						<p class="text-xs mt-1">Las capturas aparecerán aquí cuando se tomen screenshots de las cámaras</p>
					</div>
				{/if}
			{/if}

			<!-- ═══════════════ NOTIFICATIONS ═══════════════ -->
			{#if app.activeTab === 'notifications'}
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-lg font-semibold text-content">Notificaciones</h2>
					{#if app.unreadCount > 0}
						<button onclick={() => app.markAllNotificationsRead()} class="btn btn-secondary py-2 text-xs flex items-center gap-1.5">
							<CheckCheck class="w-3.5 h-3.5" /> Marcar todas como leídas
						</button>
					{/if}
				</div>
				{#if app.notifications.length > 0}
					<div class="space-y-2">
						{#each app.notifications as notif (notif.id)}
							<div class="bg-surface-alt border border-edge rounded-xl p-4 flex items-start gap-3 transition {notif.read ? 'opacity-60' : ''}" style="box-shadow: 0 1px 3px var(--th-shadow);">
								<div class="w-8 h-8 rounded-full flex items-center justify-center shrink-0
									{notif.severity === 'error' ? 'bg-red-100 dark:bg-red-900/30' : notif.severity === 'warning' ? 'bg-amber-100 dark:bg-amber-900/30' : 'bg-blue-100 dark:bg-blue-900/30'}">
									<Bell class="w-4 h-4 {notif.severity === 'error' ? 'text-red-500' : notif.severity === 'warning' ? 'text-amber-500' : 'text-blue-500'}" />
								</div>
								<div class="flex-1 min-w-0">
									<div class="flex items-center justify-between gap-2">
										<h4 class="text-sm font-medium text-content {notif.read ? '' : 'font-semibold'}">{notif.title}</h4>
										<span class="text-[10px] text-content-muted shrink-0">{notif.created_at ? new Date(notif.created_at + 'Z').toLocaleString() : ''}</span>
									</div>
									{#if notif.message}<p class="text-xs text-content-secondary mt-0.5">{notif.message}</p>{/if}
									<div class="flex items-center gap-2 mt-2">
										<span class="badge badge-neutral text-[10px] capitalize">{notif.category}</span>
										<span class="badge {notif.severity === 'error' ? 'badge-danger' : notif.severity === 'warning' ? 'badge-warn' : 'badge-info'} text-[10px]">{notif.severity}</span>
										{#if !notif.read}
											<button onclick={() => app.markNotificationRead(notif.id)} class="text-[10px] text-primary hover:underline ml-auto">Marcar leída</button>
										{/if}
										<button onclick={() => app.deleteNotification(notif.id)} class="text-[10px] text-content-muted hover:text-destructive ml-auto">
											<Trash2 class="w-3 h-3" />
										</button>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-16 text-content-muted">
						<BellOff class="w-12 h-12 mx-auto mb-3 opacity-40" />
						<p class="text-sm">No hay notificaciones</p>
					</div>
				{/if}
			{/if}

			<!-- ═══════════════ ROLES ═══════════════ -->
			{#if app.activeTab === 'roles'}
				<div class="flex items-center justify-between mb-4">
					<h2 class="text-lg font-semibold text-content">Roles y Permisos</h2>
					<button onclick={openNewRole} class="btn btn-primary py-2 text-xs">
						<Plus class="w-3.5 h-3.5" /> Nuevo Rol
					</button>
				</div>
				{#if app.roles.length > 0}
					<div class="space-y-4">
						{#each app.roles as role (role.id)}
							<div class="bg-surface-alt border border-edge rounded-xl overflow-hidden transition hover:border-edge-secondary" style="box-shadow: 0 1px 3px var(--th-shadow);">
								<div class="px-4 sm:px-5 py-4 flex items-center justify-between">
									<div class="flex items-center gap-3 min-w-0">
										<div class="w-10 h-10 rounded-lg flex items-center justify-center shrink-0" style="background:var(--th-badge-info-bg);">
											<Shield class="w-5 h-5" style="color:var(--th-badge-info-text);" />
										</div>
										<div class="min-w-0">
											<div class="flex items-center gap-2 flex-wrap">
												<h4 class="font-semibold text-content">{role.name}</h4>
												{#if role.is_system}<span class="badge badge-warn text-[10px]">Sistema</span>{/if}
											</div>
											{#if role.description}<p class="text-xs text-content-muted mt-0.5">{role.description}</p>{/if}
										</div>
									</div>
									<div class="flex items-center gap-1.5 shrink-0">
										<button onclick={() => openEditRole(role)} class="btn btn-ghost p-1.5" title="Editar">
											<Edit2 class="w-4 h-4" />
										</button>
										{#if !role.is_system}
											<button onclick={() => confirmDeleteRole(role)} class="btn btn-ghost p-1.5 text-destructive" title="Eliminar">
												<Trash2 class="w-4 h-4" />
											</button>
										{:else}
											<button disabled class="p-1.5 text-content-muted/30 cursor-not-allowed rounded" title="No se puede eliminar">
												<Lock class="w-4 h-4" />
											</button>
										{/if}
									</div>
								</div>
								<!-- Permissions grid -->
								<div class="border-t border-edge/50 bg-surface-inset overflow-x-auto">
									<table class="w-full text-xs">
										<thead>
											<tr class="text-content-muted">
												<th class="px-4 py-2 text-left font-medium">Módulo</th>
												<th class="px-2 py-2 text-center font-medium">Ver</th>
												<th class="px-2 py-2 text-center font-medium">Crear</th>
												<th class="px-2 py-2 text-center font-medium">Editar</th>
												<th class="px-2 py-2 text-center font-medium">Eliminar</th>
											</tr>
										</thead>
										<tbody class="divide-y divide-edge/30">
											{#each role.permissions as perm}
												<tr>
													<td class="px-4 py-1.5 text-content-secondary">{moduleLabels[perm.module] || perm.module}</td>
													<td class="px-2 py-1.5 text-center">{#if perm.can_view}<span class="text-emerald-500">✓</span>{:else}<span class="text-content-muted/30">—</span>{/if}</td>
													<td class="px-2 py-1.5 text-center">{#if perm.can_create}<span class="text-emerald-500">✓</span>{:else}<span class="text-content-muted/30">—</span>{/if}</td>
													<td class="px-2 py-1.5 text-center">{#if perm.can_edit}<span class="text-emerald-500">✓</span>{:else}<span class="text-content-muted/30">—</span>{/if}</td>
													<td class="px-2 py-1.5 text-center">{#if perm.can_delete}<span class="text-emerald-500">✓</span>{:else}<span class="text-content-muted/30">—</span>{/if}</td>
												</tr>
											{/each}
										</tbody>
									</table>
								</div>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-16 text-content-muted">
						<Shield class="w-12 h-12 mx-auto mb-3 opacity-40" />
						<p class="text-sm">No hay roles definidos</p>
					</div>
				{/if}
			{/if}

			</div>
		</main>
	</div>
</div>

<!-- Modals -->
<CameraModal bind:open={cameraModalOpen} {editCamera} />
<LocationModal bind:open={locationModalOpen} {editLocation} />
<AreaModal bind:open={areaModalOpen} {editArea} />
<MosaicModal bind:open={mosaicModalOpen} {editMosaic} />
<UserModal bind:open={userModalOpen} {editUser} />
<RoleModal bind:open={roleModalOpen} {editRole} />
<CameraViewer bind:open={viewerOpen} streamName={viewerStreamName} />
<ShareMosaicModal bind:open={shareModalOpen} mosaic={shareMosaic} />
<ConfirmDeleteDialog bind:open={deleteDialogOpen} title={deleteDialogTitle} message={deleteDialogMessage} onConfirm={deleteDialogAction} />
<NotificationPanel bind:open={notifPanelOpen} />
{/if}
