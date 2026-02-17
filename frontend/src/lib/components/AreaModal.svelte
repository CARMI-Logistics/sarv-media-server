<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { AreaWithLocation } from '$lib/types';
	import { z } from 'zod';
	import { AlertCircle } from 'lucide-svelte';

	let {
		open = $bindable(false),
		editArea = null as AreaWithLocation | null
	}: {
		open: boolean;
		editArea: AreaWithLocation | null;
	} = $props();

	const schema = z.object({
		name: z.string().min(1, 'El nombre es requerido').min(2, 'Mínimo 2 caracteres').max(100, 'Máximo 100 caracteres'),
		locationId: z.string().min(1, 'La ubicación es requerida'),
	});

	let name = $state('');
	let locationId = $state('');
	let description = $state('');
	let error = $state('');
	let submitted = $state(false);
	let touched = $state<Record<string, boolean>>({});

	let fieldErrors = $derived.by(() => {
		const result = schema.safeParse({ name, locationId });
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

	$effect(() => {
		if (open) {
			error = '';
			submitted = false;
			touched = {};
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
		submitted = true;
		error = '';
		if (!isValid) return;
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
	<form onsubmit={handleSubmit} class="space-y-4" novalidate>
		<div>
			<label for="area-name" class="block text-sm text-content-secondary mb-1">Nombre *</label>
			<input id="area-name" type="text" bind:value={name} onblur={() => (touched.name = true)}
				class="input {showErr('name') ? 'input-error' : ''}" placeholder="ej: Entrance" />
			{#if showErr('name')}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('name')}</p>{/if}
		</div>
		<div>
			<label for="area-loc" class="block text-sm text-content-secondary mb-1">Ubicación *</label>
			<select id="area-loc" bind:value={locationId} onblur={() => (touched.locationId = true)}
				class="input {showErr('locationId') ? 'input-error' : ''}">
				<option value="">Seleccionar ubicación</option>
				{#each app.locations as loc}
					<option value={String(loc.id)}>{loc.name}</option>
				{/each}
			</select>
			{#if showErr('locationId')}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('locationId')}</p>{/if}
		</div>
		<div>
			<label for="area-desc" class="block text-sm text-content-secondary mb-1">Descripción</label>
			<textarea id="area-desc" bind:value={description} rows={3} class="input" placeholder="Descripción opcional del área"></textarea>
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
