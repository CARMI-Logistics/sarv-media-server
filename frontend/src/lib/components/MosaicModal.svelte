<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { Mosaic } from '$lib/types';
	import { X, AlertCircle, Search, MapPin, Layers, GripVertical, Video } from 'lucide-svelte';
	import { z } from 'zod';

	let {
		open = $bindable(false),
		editMosaic = null as Mosaic | null
	}: {
		open: boolean;
		editMosaic: Mosaic | null;
	} = $props();

	const schema = z.object({
		name: z.string().min(1, 'El nombre es requerido').min(2, 'Mínimo 2 caracteres').max(100, 'Máximo 100 caracteres'),
	});

	let name = $state('');
	let layout = $state('2x2');
	let selectedCameras = $state<{ id: number; name: string }[]>([]);
	let searchFilter = $state('');
	let filterLocation = $state('');
	let filterArea = $state('');
	let error = $state('');
	let submitted = $state(false);
	let touched = $state<Record<string, boolean>>({});

	// Drag-and-drop state
	let dragIndex = $state<number | null>(null);
	let dragOverIndex = $state<number | null>(null);

	let fieldErrors = $derived.by(() => {
		const result = schema.safeParse({ name });
		if (result.success) return {} as Record<string, string>;
		const errs: Record<string, string> = {};
		for (const issue of result.error.issues) {
			const key = String(issue.path[0]);
			if (!errs[key]) errs[key] = issue.message;
		}
		return errs;
	});

	let isValid = $derived(Object.keys(fieldErrors).length === 0);

	function showErr(field: string): string {
		if ((touched[field] || submitted) && fieldErrors[field]) return fieldErrors[field];
		return '';
	}

	let cols = $derived(parseInt(layout.split('x')[0]) || 2);
	let rows = $derived(parseInt(layout.split('x')[1]) || 2);
	let maxCams = $derived(cols * rows);

	let filterAreas = $derived(
		filterLocation ? app.areas.filter((a) => a.location_id === Number(filterLocation)) : []
	);

	let availableCameras = $derived.by(() => {
		let cams = app.cameras.filter((c) => c.enabled);
		if (searchFilter) {
			const q = searchFilter.toLowerCase();
			cams = cams.filter((c) => c.name.toLowerCase().includes(q) || c.host.includes(q));
		}
		if (filterLocation) {
			const loc = app.locations.find((l) => l.id === Number(filterLocation));
			if (loc) cams = cams.filter((c) => c.location === loc.name);
		}
		if (filterArea) {
			cams = cams.filter((c) => c.area === filterArea);
		}
		return cams;
	});

	// Grid cells: fill selected cameras into grid slots, rest are empty
	let gridCells = $derived.by(() => {
		const cells: ({ id: number; name: string } | null)[] = [];
		for (let i = 0; i < maxCams; i++) {
			cells.push(selectedCameras[i] ?? null);
		}
		return cells;
	});

	$effect(() => {
		if (open) {
			error = '';
			submitted = false;
			touched = {};
			searchFilter = '';
			filterLocation = '';
			filterArea = '';
			dragIndex = null;
			dragOverIndex = null;
			if (editMosaic) {
				name = editMosaic.name;
				layout = editMosaic.layout;
				selectedCameras = editMosaic.cameras.map((c) => ({ id: c.camera_id, name: c.camera_name }));
			} else {
				name = '';
				layout = '2x2';
				selectedCameras = [];
			}
		}
	});

	function toggleCamera(id: number, camName: string, checked: boolean) {
		if (checked) {
			if (selectedCameras.length >= maxCams) {
				error = `Máximo ${maxCams} cámaras para layout ${layout}`;
				return;
			}
			selectedCameras = [...selectedCameras, { id, name: camName }];
		} else {
			selectedCameras = selectedCameras.filter((c) => c.id !== id);
		}
		error = '';
	}

	function removeCamera(id: number) {
		selectedCameras = selectedCameras.filter((c) => c.id !== id);
	}

	function removeCameraAt(index: number) {
		selectedCameras = selectedCameras.filter((_, i) => i !== index);
	}

	function onLayoutChange() {
		if (selectedCameras.length > maxCams) {
			selectedCameras = selectedCameras.slice(0, maxCams);
		}
	}

	// Drag-and-drop handlers
	function onDragStart(index: number) {
		dragIndex = index;
	}

	function onDragOver(e: DragEvent, index: number) {
		e.preventDefault();
		dragOverIndex = index;
	}

	function onDrop(e: DragEvent, targetIndex: number) {
		e.preventDefault();
		if (dragIndex === null || dragIndex === targetIndex) { dragIndex = null; dragOverIndex = null; return; }
		const items = [...selectedCameras];
		const [moved] = items.splice(dragIndex, 1);
		items.splice(targetIndex > dragIndex ? targetIndex - 1 : targetIndex, 0, moved);
		selectedCameras = items;
		dragIndex = null;
		dragOverIndex = null;
	}

	function onDragEnd() {
		dragIndex = null;
		dragOverIndex = null;
	}

	// Touch drag-and-drop for mobile
	let touchStartIndex = $state<number | null>(null);
	let touchCurrentY = $state(0);

	function onTouchStart(index: number) {
		touchStartIndex = index;
	}

	function onTouchMove(index: number, targetIndex: number) {
		if (touchStartIndex !== null && touchStartIndex !== targetIndex) {
			const items = [...selectedCameras];
			const [moved] = items.splice(touchStartIndex, 1);
			items.splice(targetIndex, 0, moved);
			selectedCameras = items;
			touchStartIndex = targetIndex;
		}
	}

	function onTouchEnd() {
		touchStartIndex = null;
	}

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		submitted = true;
		error = '';
		if (!isValid) return;
		if (!selectedCameras.length) {
			error = 'Selecciona al menos una cámara';
			return;
		}
		const result = await app.saveMosaic(editMosaic?.id ?? null, {
			name: name.trim(),
			layout,
			camera_ids: selectedCameras.map((c) => c.id)
		});
		if (result === true) open = false;
		else if (typeof result === 'string') error = result;
	}
</script>

<Modal bind:open title={editMosaic ? 'Editar Mosaico' : 'Nuevo Mosaico'} maxWidth="max-w-4xl">
	<form onsubmit={handleSubmit} class="space-y-4" novalidate>
		<!-- Row 1: Name + Layout -->
		<div class="grid grid-cols-2 gap-4">
			<div>
				<label for="mos-name" class="block text-sm text-content-secondary mb-1">Nombre *</label>
				<input id="mos-name" type="text" bind:value={name} onblur={() => (touched.name = true)}
					class="input {showErr('name') ? 'input-error' : ''}" placeholder="ej: mosaic-entrance" />
				{#if showErr('name')}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('name')}</p>{/if}
			</div>
			<div>
				<label for="mos-layout" class="block text-sm text-content-secondary mb-1">Layout</label>
				<select id="mos-layout" bind:value={layout} onchange={onLayoutChange} class="input">
					<option value="1x1">1x1 (1 cámara)</option>
					<option value="2x2">2x2 (4 cámaras)</option>
					<option value="3x3">3x3 (9 cámaras)</option>
					<option value="4x4">4x4 (16 cámaras)</option>
					<option value="5x5">5x5 (25 cámaras)</option>
					<option value="6x6">6x6 (36 cámaras)</option>
				</select>
			</div>
		</div>

		<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
			<!-- Left: Camera selector with filters -->
			<div class="space-y-2">
				<span class="block text-sm font-medium text-content">Seleccionar cámaras</span>
				<!-- Search -->
				<div class="relative">
					<Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-content-muted" />
					<input type="text" bind:value={searchFilter} class="input pl-8 py-2 text-sm" placeholder="Buscar por nombre o IP..." />
				</div>
				<!-- Filters row -->
				<div class="flex gap-2">
					<select bind:value={filterLocation} onchange={() => { filterArea = ''; }} class="input py-1.5 text-xs flex-1">
						<option value=""><MapPin class="w-3 h-3" /> Todas las ubicaciones</option>
						{#each app.locations as loc}
							<option value={String(loc.id)}>{loc.name}</option>
						{/each}
					</select>
					<select bind:value={filterArea} class="input py-1.5 text-xs flex-1" disabled={!filterLocation}>
						<option value="">Todas las áreas</option>
						{#each filterAreas as area}
							<option value={area.name}>{area.name}</option>
						{/each}
					</select>
				</div>
				<!-- Camera list -->
				<div class="max-h-52 overflow-y-auto space-y-0.5 bg-surface-raised rounded-lg p-1.5 border border-edge">
					{#if availableCameras.length === 0}
						<p class="text-xs text-content-muted text-center py-4">No se encontraron cámaras</p>
					{:else}
						{#each availableCameras as cam}
							{@const isSelected = selectedCameras.some((s) => s.id === cam.id)}
							{@const status = app.cameraStatuses[cam.name] ?? 'unknown'}
							<label class="flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-surface-hover cursor-pointer text-xs transition {isSelected ? 'bg-primary/10 ring-1 ring-primary/30' : ''}">
								<input type="checkbox" checked={isSelected}
									onchange={(e) => toggleCamera(cam.id, cam.name, (e.target as HTMLInputElement).checked)}
									class="rounded bg-surface-raised border-edge text-blue-600 w-3.5 h-3.5" />
								<span class="status-dot {status === 'online' ? 'on' : status === 'offline' ? 'off' : 'bg-gray-400'} w-1.5 h-1.5 shrink-0"></span>
								<span class="text-content font-medium truncate">{cam.name}</span>
								<span class="text-content-muted ml-auto shrink-0 text-[10px]">{cam.location || ''}{cam.area ? ` / ${cam.area}` : ''}</span>
							</label>
						{/each}
					{/if}
				</div>
				<p class="text-[10px] text-content-muted">{availableCameras.length} cámaras disponibles · {selectedCameras.length}/{maxCams} seleccionadas</p>
			</div>

			<!-- Right: Visual grid preview with drag-and-drop -->
			<div class="space-y-2">
				<span class="block text-sm font-medium text-content">Disposición del mosaico <span class="text-content-muted font-normal">(arrastra para reordenar)</span></span>
				<div class="grid gap-1.5 bg-surface-raised rounded-lg p-2 border border-edge"
					style="grid-template-columns: repeat({cols}, 1fr);">
					{#each gridCells as cell, idx}
						{#if cell}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class="aspect-video rounded-md border-2 flex flex-col items-center justify-center gap-0.5 text-[10px] cursor-grab active:cursor-grabbing transition-all select-none
									{dragOverIndex === idx ? 'border-primary bg-primary/20 scale-105' : 'border-blue-500/40 bg-blue-500/10'}
									{dragIndex === idx ? 'opacity-40 scale-95' : ''}"
								draggable="true"
								ondragstart={() => onDragStart(idx)}
								ondragover={(e) => onDragOver(e, idx)}
								ondrop={(e) => onDrop(e, idx)}
								ondragend={onDragEnd}
								ontouchstart={() => onTouchStart(idx)}
								ontouchend={onTouchEnd}
								role="listitem"
							>
								<GripVertical class="w-3 h-3 text-content-muted/50" />
								<Video class="w-3.5 h-3.5 text-blue-400" />
								<span class="text-content font-medium truncate max-w-full px-1 text-center leading-tight">{cell.name}</span>
								<button type="button" onclick={() => removeCameraAt(idx)}
									class="text-content-muted hover:text-destructive transition mt-0.5">
									<X class="w-3 h-3" />
								</button>
							</div>
						{:else}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class="aspect-video rounded-md border-2 border-dashed border-edge flex items-center justify-center text-[10px] text-content-muted/50 transition-all
									{dragOverIndex === idx ? 'border-primary bg-primary/10' : ''}"
								ondragover={(e) => onDragOver(e, idx)}
								ondrop={(e) => onDrop(e, idx)}
								role="listitem"
							>
								{idx + 1}
							</div>
						{/if}
					{/each}
				</div>
				<p class="text-[10px] text-content-muted">El orden en la cuadrícula determina la posición en el mosaico final</p>
			</div>
		</div>

		{#if error}
			<div class="flex items-center gap-2 text-sm rounded-lg px-3.5 py-2.5 border"
				style="background: var(--th-badge-danger-bg); color: var(--th-badge-danger-text); border-color: var(--th-badge-danger-bg);">
				<AlertCircle class="w-4 h-4 shrink-0" /><span>{error}</span>
			</div>
		{/if}
		<div class="flex justify-end gap-3 pt-2">
			<button type="button" onclick={() => (open = false)} class="btn btn-secondary">Cancelar</button>
			<button type="submit" class="btn btn-primary">Guardar</button>
		</div>
	</form>
</Modal>
