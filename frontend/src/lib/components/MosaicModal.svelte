<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { Mosaic } from '$lib/types';
	import { X } from 'lucide-svelte';

	let {
		open = $bindable(false),
		editMosaic = null as Mosaic | null
	}: {
		open: boolean;
		editMosaic: Mosaic | null;
	} = $props();

	let name = $state('');
	let layout = $state('2x2');
	let selectedCameras = $state<{ id: number; name: string }[]>([]);
	let searchFilter = $state('');
	let error = $state('');

	let maxCameras = $derived(() => {
		const [c, r] = layout.split('x').map(Number);
		return c * r;
	});

	let availableCameras = $derived(() => {
		let cams = app.cameras.filter((c) => c.enabled);
		if (searchFilter) {
			const q = searchFilter.toLowerCase();
			cams = cams.filter((c) => c.name.toLowerCase().includes(q) || c.host.includes(q));
		}
		return cams;
	});

	$effect(() => {
		if (open) {
			error = '';
			searchFilter = '';
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
			if (selectedCameras.length >= maxCameras()) {
				error = `Máximo ${maxCameras()} cámaras para layout ${layout}`;
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

	function onLayoutChange() {
		const max = maxCameras();
		if (selectedCameras.length > max) {
			selectedCameras = selectedCameras.slice(0, max);
		}
	}

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
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

<Modal bind:open title={editMosaic ? 'Editar Mosaico' : 'Nuevo Mosaico'} maxWidth="max-w-2xl">
	<form onsubmit={handleSubmit} class="space-y-4">
		<div class="grid grid-cols-2 gap-4">
			<div>
				<label for="mos-name" class="block text-sm text-gray-400 mb-1">Nombre *</label>
				<input id="mos-name" type="text" bind:value={name} required
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
					placeholder="ej: mosaic-entrance" />
			</div>
			<div>
				<label for="mos-layout" class="block text-sm text-gray-400 mb-1">Layout</label>
				<select id="mos-layout" bind:value={layout} onchange={onLayoutChange}
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500">
					<option value="1x1">1x1 (1 cámara)</option>
					<option value="2x2">2x2 (4 cámaras)</option>
					<option value="3x3">3x3 (9 cámaras)</option>
					<option value="4x4">4x4 (16 cámaras)</option>
					<option value="5x5">5x5 (25 cámaras)</option>
					<option value="6x6">6x6 (36 cámaras)</option>
				</select>
			</div>
		</div>

		<div>
			<label for="mos-search" class="block text-sm text-gray-400 mb-1">Buscar cámaras</label>
			<input id="mos-search" type="text" bind:value={searchFilter}
				class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 mb-2"
				placeholder="Filtrar cámaras..." />
			<div class="max-h-48 overflow-y-auto space-y-1 bg-gray-800 rounded-lg p-2 border border-gray-700">
				{#each availableCameras() as cam}
					{@const isSelected = selectedCameras.some((s) => s.id === cam.id)}
					<label class="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-gray-700 cursor-pointer text-sm {isSelected ? 'bg-blue-900/20' : ''}">
						<input type="checkbox" checked={isSelected}
							onchange={(e) => toggleCamera(cam.id, cam.name, (e.target as HTMLInputElement).checked)}
							class="rounded bg-gray-700 border-gray-600 text-blue-600" />
						<span>{cam.name}</span>
						<span class="text-gray-500 text-xs ml-auto">{cam.host}</span>
					</label>
				{/each}
			</div>
		</div>

		<div>
			<span class="block text-sm text-gray-400 mb-1">
				Cámaras seleccionadas ({selectedCameras.length}/{maxCameras()})
			</span>
			<div class="min-h-[40px] bg-gray-800 rounded-lg p-2 border border-gray-700 flex flex-wrap gap-2">
				{#if selectedCameras.length === 0}
					<span class="text-gray-500 text-sm">Selecciona cámaras de la lista</span>
				{:else}
					{#each selectedCameras as cam}
						<span class="inline-flex items-center gap-1 px-2 py-1 bg-blue-900/30 text-blue-300 rounded text-xs">
							{cam.name}
							<button type="button" onclick={() => removeCamera(cam.id)} class="text-blue-400 hover:text-red-400">
								<X class="w-3 h-3" />
							</button>
						</span>
					{/each}
				{/if}
			</div>
		</div>

		{#if error}
			<div class="text-red-400 text-sm bg-red-900/20 border border-red-800 rounded-lg px-3 py-2">{error}</div>
		{/if}
		<div class="flex justify-end gap-3 pt-2">
			<button type="button" onclick={() => (open = false)} class="px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-lg text-sm transition">Cancelar</button>
			<button type="submit" class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-sm font-medium transition">Guardar</button>
		</div>
	</form>
</Modal>
