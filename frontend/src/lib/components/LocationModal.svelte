<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { Location } from '$lib/types';
	import { z } from 'zod';
	import { AlertCircle } from 'lucide-svelte';

	let {
		open = $bindable(false),
		editLocation = null as Location | null
	}: {
		open: boolean;
		editLocation: Location | null;
	} = $props();

	const schema = z.object({
		name: z.string().min(1, 'El nombre es requerido').min(2, 'Mínimo 2 caracteres').max(100, 'Máximo 100 caracteres'),
	});

	let name = $state('');
	let description = $state('');
	let isSystem = $state(false);
	let error = $state('');
	let submitted = $state(false);
	let touched = $state<Record<string, boolean>>({});

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

	$effect(() => {
		if (open) {
			error = '';
			submitted = false;
			touched = {};
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
		submitted = true;
		error = '';
		if (!isValid) return;
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
	<form onsubmit={handleSubmit} class="space-y-4" novalidate>
		<div>
			<label for="loc-name" class="block text-sm text-content-secondary mb-1">Nombre *</label>
			<input id="loc-name" type="text" bind:value={name} onblur={() => (touched.name = true)}
				class="input {showErr('name') ? 'input-error' : ''}" placeholder="ej: Warehouse 1" />
			{#if showErr('name')}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('name')}</p>{/if}
		</div>
		<div>
			<label for="loc-desc" class="block text-sm text-content-secondary mb-1">Descripción</label>
			<textarea id="loc-desc" bind:value={description} rows={3} class="input" placeholder="Descripción opcional de la ubicación"></textarea>
		</div>
		<div>
			<label class="flex items-center gap-2 text-sm cursor-pointer">
				<input type="checkbox" bind:checked={isSystem} class="rounded bg-surface-raised border-edge text-blue-600 focus:ring-blue-500" />
				<span class="text-content-secondary">Ubicación del sistema <span class="text-xs text-content-muted">(no se puede eliminar)</span></span>
			</label>
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
