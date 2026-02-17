<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { Camera } from '$lib/types';
	import { z } from 'zod';
	import { AlertCircle } from 'lucide-svelte';

	let {
		open = $bindable(false),
		editCamera = null as Camera | null
	}: {
		open: boolean;
		editCamera: Camera | null;
	} = $props();

	const schema = z.object({
		name: z.string().min(1, 'El nombre es requerido').min(2, 'Mínimo 2 caracteres').max(100, 'Máximo 100 caracteres'),
		host: z.string().min(1, 'El host es requerido').min(2, 'Mínimo 2 caracteres'),
		port: z.number().min(1, 'Puerto inválido').max(65535, 'Puerto máximo 65535'),
	});

	let name = $state('');
	let host = $state('');
	let port = $state(554);
	let username = $state('');
	let password = $state('');
	let path = $state('/defaultPrimary?streamType=m');
	let protocol = $state('rtsp');
	let locationId = $state('');
	let area = $state('');
	let enabled = $state(true);
	let record = $state(true);
	let sourceOnDemand = $state(false);
	let error = $state('');
	let submitted = $state(false);
	let touched = $state<Record<string, boolean>>({});

	let fieldErrors = $derived.by(() => {
		const result = schema.safeParse({ name, host, port });
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

	let areasForLocation = $derived(
		locationId ? app.areas.filter((a) => a.location_id === Number(locationId)) : []
	);

	$effect(() => {
		if (open) {
			error = '';
			submitted = false;
			touched = {};
			if (editCamera) {
				name = editCamera.name;
				host = editCamera.host;
				port = editCamera.port;
				username = editCamera.username || '';
				password = editCamera.password || '';
				path = editCamera.path;
				protocol = editCamera.protocol;
				enabled = editCamera.enabled;
				record = editCamera.record;
				sourceOnDemand = editCamera.source_on_demand;
				const loc = app.locations.find((l) => l.name === editCamera.location);
				locationId = loc ? String(loc.id) : '';
				const savedArea = editCamera.area;
				setTimeout(() => { area = savedArea || ''; }, 50);
			} else {
				name = '';
				host = '';
				port = 554;
				username = '';
				password = '';
				path = '/defaultPrimary?streamType=m';
				protocol = 'rtsp';
				locationId = '';
				area = '';
				enabled = true;
				record = true;
				sourceOnDemand = false;
			}
		}
	});

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		submitted = true;
		error = '';
		if (!isValid) return;
		const loc = app.locations.find((l) => l.id === Number(locationId));
		const body = {
			name: name.trim(),
			host: host.trim(),
			port: port || 554,
			username: username.trim(),
			password: password.trim(),
			path: path.trim(),
			protocol,
			enabled,
			record,
			source_on_demand: sourceOnDemand,
			location: loc ? loc.name : '',
			area: area.trim()
		};
		const ok = await app.saveCamera(editCamera?.id ?? null, body);
		if (ok) open = false;
		else error = 'Error guardando cámara';
	}
</script>

<Modal bind:open title={editCamera ? 'Editar Cámara' : 'Nueva Cámara'}>
	<form onsubmit={handleSubmit} class="space-y-4" novalidate>
		<div>
			<label for="cam-name" class="block text-sm text-content-secondary mb-1">Nombre *</label>
			<input id="cam-name" type="text" bind:value={name} onblur={() => (touched.name = true)}
				class="input {showErr('name') ? 'input-error' : ''}" placeholder="ej: entrance-camera-1" />
			{#if showErr('name')}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('name')}</p>{/if}
		</div>
		<div class="grid grid-cols-3 gap-3">
			<div class="col-span-2">
				<label for="cam-host" class="block text-sm text-content-secondary mb-1">Host / IP *</label>
				<input id="cam-host" type="text" bind:value={host} onblur={() => (touched.host = true)}
					class="input {showErr('host') ? 'input-error' : ''}" placeholder="10.0.0.30" />
				{#if showErr('host')}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('host')}</p>{/if}
			</div>
			<div>
				<label for="cam-port" class="block text-sm text-content-secondary mb-1">Puerto</label>
				<input id="cam-port" type="number" bind:value={port} onblur={() => (touched.port = true)}
					class="input {showErr('port') ? 'input-error' : ''}" />
				{#if showErr('port')}<p class="text-xs text-red-500 mt-1">{showErr('port')}</p>{/if}
			</div>
		</div>
		<div class="grid grid-cols-2 gap-3">
			<div>
				<label for="cam-user" class="block text-sm text-content-secondary mb-1">Usuario</label>
				<input id="cam-user" type="text" bind:value={username} class="input" placeholder="admin" />
			</div>
			<div>
				<label for="cam-pass" class="block text-sm text-content-secondary mb-1">Contraseña</label>
				<input id="cam-pass" type="text" bind:value={password} class="input" placeholder="admin123" />
			</div>
		</div>
		<div>
			<label for="cam-path" class="block text-sm text-content-secondary mb-1">Path RTSP</label>
			<input id="cam-path" type="text" bind:value={path} class="input" />
		</div>
		<div class="grid grid-cols-2 gap-3">
			<div>
				<label for="cam-protocol" class="block text-sm text-content-secondary mb-1">Protocolo</label>
				<select id="cam-protocol" bind:value={protocol} class="input">
					<option value="rtsp">RTSP</option>
					<option value="rtmp">RTMP</option>
				</select>
			</div>
			<div>
				<label for="cam-location" class="block text-sm text-content-secondary mb-1">Ubicación</label>
				<select id="cam-location" bind:value={locationId} class="input">
					<option value="">Seleccionar ubicación</option>
					{#each app.locations as loc}
						<option value={String(loc.id)}>{loc.name}</option>
					{/each}
				</select>
			</div>
		</div>
		<div>
			<label for="cam-area" class="block text-sm text-content-secondary mb-1">Área</label>
			<select id="cam-area" bind:value={area} class="input">
				{#if !locationId}
					<option value="">Seleccionar área (primero elige ubicación)</option>
				{:else}
					<option value="">Seleccionar área</option>
					{#each areasForLocation as a}
						<option value={a.name}>{a.name}</option>
					{/each}
				{/if}
			</select>
		</div>
		<div class="grid grid-cols-3 gap-3">
			<label class="flex items-center gap-2 text-sm text-content-secondary cursor-pointer">
				<input type="checkbox" bind:checked={enabled} class="rounded bg-surface-raised border-edge text-blue-600 focus:ring-blue-500" />
				Habilitada
			</label>
			<label class="flex items-center gap-2 text-sm text-content-secondary cursor-pointer">
				<input type="checkbox" bind:checked={record} class="rounded bg-surface-raised border-edge text-blue-600 focus:ring-blue-500" />
				Grabar
			</label>
			<label class="flex items-center gap-2 text-sm text-content-secondary cursor-pointer">
				<input type="checkbox" bind:checked={sourceOnDemand} class="rounded bg-surface-raised border-edge text-blue-600 focus:ring-blue-500" />
				On-demand
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
