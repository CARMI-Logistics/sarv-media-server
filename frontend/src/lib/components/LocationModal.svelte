<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { Location } from '$lib/types';

	let {
		open = $bindable(false),
		editLocation = null as Location | null
	}: {
		open: boolean;
		editLocation: Location | null;
	} = $props();

	let name = $state('');
	let description = $state('');
	let isSystem = $state(false);
	let error = $state('');

	$effect(() => {
		if (open) {
			error = '';
			if (editLocation) {
				name = editLocation.name;
				description = editLocation.description || '';
				isSystem = editLocation.is_system || false;
			} else {
				name = '';
				description = '';
				isSystem = false;
			}
		}
	});

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
		const result = await app.saveLocation(editLocation?.id ?? null, {
			name: name.trim(),
			description: description.trim(),
			is_system: isSystem
		});
		if (result === true) open = false;
		else if (typeof result === 'string') error = result;
	}
</script>

<Modal bind:open title={editLocation ? 'Editar Ubicación' : 'Nueva Ubicación'} maxWidth="max-w-md">
	<form onsubmit={handleSubmit} class="space-y-4">
		<div>
			<label for="loc-name" class="block text-sm text-gray-400 mb-1">Nombre *</label>
			<input id="loc-name" type="text" bind:value={name} required
				class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
				placeholder="ej: Warehouse 1" />
		</div>
		<div>
			<label for="loc-desc" class="block text-sm text-gray-400 mb-1">Descripción</label>
			<textarea id="loc-desc" bind:value={description} rows={3}
				class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
				placeholder="Descripción opcional de la ubicación"></textarea>
		</div>
		<div>
			<label class="flex items-center gap-2 text-sm cursor-pointer">
				<input type="checkbox" bind:checked={isSystem} class="rounded bg-gray-700 border-gray-600 text-blue-600 focus:ring-blue-500" />
				<span class="text-gray-300">Ubicación del sistema <span class="text-xs text-gray-500">(no se puede eliminar)</span></span>
			</label>
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
