<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { AreaWithLocation } from '$lib/types';

	let {
		open = $bindable(false),
		editArea = null as AreaWithLocation | null
	}: {
		open: boolean;
		editArea: AreaWithLocation | null;
	} = $props();

	let name = $state('');
	let locationId = $state('');
	let description = $state('');
	let error = $state('');

	$effect(() => {
		if (open) {
			error = '';
			if (editArea && editArea.id > 0) {
				name = editArea.name;
				locationId = String(editArea.location_id);
				description = editArea.description || '';
			} else if (editArea && editArea.id === 0) {
				name = '';
				locationId = editArea.location_id ? String(editArea.location_id) : '';
				description = '';
			} else {
				name = '';
				locationId = '';
				description = '';
			}
		}
	});

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		const areaId = editArea && editArea.id > 0 ? editArea.id : null;
		const result = await app.saveArea(areaId, {
			name: name.trim(),
			location_id: Number(locationId),
			description: description.trim()
		});
		if (result === true) open = false;
		else if (typeof result === 'string') error = result;
	}
</script>

<Modal bind:open title={editArea && editArea.id > 0 ? 'Editar Área' : 'Nueva Área'} maxWidth="max-w-md">
	<form onsubmit={handleSubmit} class="space-y-4">
		<div>
			<label for="area-name" class="block text-sm text-gray-400 mb-1">Nombre *</label>
			<input id="area-name" type="text" bind:value={name} required
				class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
				placeholder="ej: Entrance" />
		</div>
		<div>
			<label for="area-loc" class="block text-sm text-gray-400 mb-1">Ubicación *</label>
			<select id="area-loc" bind:value={locationId} required
				class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500">
				<option value="">Seleccionar ubicación</option>
				{#each app.locations as loc}
					<option value={String(loc.id)}>{loc.name}</option>
				{/each}
			</select>
		</div>
		<div>
			<label for="area-desc" class="block text-sm text-gray-400 mb-1">Descripción</label>
			<textarea id="area-desc" bind:value={description} rows={3}
				class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
				placeholder="Descripción opcional del área"></textarea>
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
