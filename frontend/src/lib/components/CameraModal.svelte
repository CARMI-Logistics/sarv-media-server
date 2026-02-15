<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { Camera } from '$lib/types';

	let {
		open = $bindable(false),
		editCamera = null as Camera | null
	}: {
		open: boolean;
		editCamera: Camera | null;
	} = $props();

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

	let areasForLocation = $derived(
		locationId ? app.areas.filter((a) => a.location_id === Number(locationId)) : []
	);

	$effect(() => {
		if (open) {
			error = '';
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
				// Set area after a tick so the derived areasForLocation updates
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
		error = '';
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
	<form onsubmit={handleSubmit} class="space-y-4">
		<div>
			<label for="cam-name" class="block text-sm text-gray-400 mb-1">Nombre *</label>
			<input id="cam-name" type="text" bind:value={name} required
				class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
				placeholder="ej: entrance-camera-1" />
		</div>
		<div class="grid grid-cols-3 gap-3">
			<div class="col-span-2">
				<label for="cam-host" class="block text-sm text-gray-400 mb-1">Host / IP *</label>
				<input id="cam-host" type="text" bind:value={host} required
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
					placeholder="10.0.0.30" />
			</div>
			<div>
				<label for="cam-port" class="block text-sm text-gray-400 mb-1">Puerto</label>
				<input id="cam-port" type="number" bind:value={port}
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500" />
			</div>
		</div>
		<div class="grid grid-cols-2 gap-3">
			<div>
				<label for="cam-user" class="block text-sm text-gray-400 mb-1">Usuario</label>
				<input id="cam-user" type="text" bind:value={username}
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
					placeholder="admin" />
			</div>
			<div>
				<label for="cam-pass" class="block text-sm text-gray-400 mb-1">Contraseña</label>
				<input id="cam-pass" type="text" bind:value={password}
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
					placeholder="admin123" />
			</div>
		</div>
		<div>
			<label for="cam-path" class="block text-sm text-gray-400 mb-1">Path RTSP</label>
			<input id="cam-path" type="text" bind:value={path}
				class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500" />
		</div>
		<div class="grid grid-cols-2 gap-3">
			<div>
				<label for="cam-protocol" class="block text-sm text-gray-400 mb-1">Protocolo</label>
				<select id="cam-protocol" bind:value={protocol}
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500">
					<option value="rtsp">RTSP</option>
					<option value="rtmp">RTMP</option>
				</select>
			</div>
			<div>
				<label for="cam-location" class="block text-sm text-gray-400 mb-1">Ubicación</label>
				<select id="cam-location" bind:value={locationId}
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500">
					<option value="">Seleccionar ubicación</option>
					{#each app.locations as loc}
						<option value={String(loc.id)}>{loc.name}</option>
					{/each}
				</select>
			</div>
		</div>
		<div>
			<label for="cam-area" class="block text-sm text-gray-400 mb-1">Área</label>
			<select id="cam-area" bind:value={area}
				class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500">
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
			<label class="flex items-center gap-2 text-sm cursor-pointer">
				<input type="checkbox" bind:checked={enabled} class="rounded bg-gray-700 border-gray-600 text-blue-600 focus:ring-blue-500" />
				Habilitada
			</label>
			<label class="flex items-center gap-2 text-sm cursor-pointer">
				<input type="checkbox" bind:checked={record} class="rounded bg-gray-700 border-gray-600 text-blue-600 focus:ring-blue-500" />
				Grabar
			</label>
			<label class="flex items-center gap-2 text-sm cursor-pointer">
				<input type="checkbox" bind:checked={sourceOnDemand} class="rounded bg-gray-700 border-gray-600 text-blue-600 focus:ring-blue-500" />
				On-demand
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
