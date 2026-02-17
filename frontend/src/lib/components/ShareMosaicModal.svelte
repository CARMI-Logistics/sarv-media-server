<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { Mosaic } from '$lib/types';
	import { z } from 'zod';
	import { AlertCircle, X, Plus, Clock, Calendar, Link, Copy, Check, Trash2, ToggleLeft, ToggleRight } from 'lucide-svelte';

	let {
		open = $bindable(false),
		mosaic = null as Mosaic | null
	}: {
		open: boolean;
		mosaic: Mosaic | null;
	} = $props();

	const emailSchema = z.string().email('Email no válido');

	let emailInput = $state('');
	let emails = $state<string[]>([]);
	let durationHours = $state(24);
	let scheduleEnabled = $state(false);
	let scheduleStart = $state('08:00');
	let scheduleEnd = $state('18:00');
	let error = $state('');
	let emailError = $state('');
	let copiedToken = $state('');

	$effect(() => {
		if (open && mosaic) {
			emailInput = '';
			emails = [];
			durationHours = 24;
			scheduleEnabled = false;
			scheduleStart = '08:00';
			scheduleEnd = '18:00';
			error = '';
			emailError = '';
			copiedToken = '';
			app.loadShares(mosaic.id);
		}
	});

	function addEmail() {
		const trimmed = emailInput.trim();
		if (!trimmed) return;
		const result = emailSchema.safeParse(trimmed);
		if (!result.success) {
			emailError = result.error.issues[0]?.message || 'Email no válido';
			return;
		}
		if (emails.includes(trimmed)) {
			emailError = 'Este email ya fue agregado';
			return;
		}
		emails = [...emails, trimmed];
		emailInput = '';
		emailError = '';
	}

	function removeEmail(email: string) {
		emails = emails.filter((e) => e !== email);
	}

	function onEmailKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			addEmail();
		}
	}

	async function handleCreate() {
		error = '';
		if (emails.length === 0) {
			error = 'Agrega al menos un email';
			return;
		}
		if (!mosaic) return;

		const data: { mosaic_id: number; emails: string[]; duration_hours: number; schedule_start?: string; schedule_end?: string } = {
			mosaic_id: mosaic.id,
			emails,
			duration_hours: durationHours,
		};
		if (scheduleEnabled) {
			data.schedule_start = scheduleStart;
			data.schedule_end = scheduleEnd;
		}

		const result = await app.createShare(data);
		if (result === true) {
			emails = [];
			emailInput = '';
			if (mosaic) app.loadShares(mosaic.id);
		} else {
			error = result;
		}
	}

	function copyShareLink(token: string) {
		const url = `${globalThis.location?.origin}/shared/${token}`;
		navigator.clipboard?.writeText(url).then(() => {
			copiedToken = token;
			setTimeout(() => { copiedToken = ''; }, 2000);
		});
	}

	const durationOptions = [
		{ value: 1, label: '1 hora' },
		{ value: 6, label: '6 horas' },
		{ value: 12, label: '12 horas' },
		{ value: 24, label: '1 día' },
		{ value: 72, label: '3 días' },
		{ value: 168, label: '1 semana' },
		{ value: 720, label: '1 mes' },
	];

	let mosaicShares = $derived(app.shares.filter((s) => s.mosaic_id === mosaic?.id));
</script>

<Modal bind:open title="Compartir Mosaico: {mosaic?.name || ''}" maxWidth="max-w-lg">
	<div class="space-y-4">
		<!-- Emails input -->
		<div>
			<label for="share-email" class="block text-sm text-content-secondary mb-1">Emails de destinatarios</label>
			<div class="flex gap-2">
				<input id="share-email" type="email" bind:value={emailInput} onkeydown={onEmailKeydown}
					class="input flex-1 py-2 text-sm {emailError ? 'input-error' : ''}" placeholder="correo@ejemplo.com" />
				<button type="button" onclick={addEmail} class="btn btn-secondary py-2 px-3">
					<Plus class="w-4 h-4" />
				</button>
			</div>
			{#if emailError}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{emailError}</p>{/if}
			{#if emails.length > 0}
				<div class="flex flex-wrap gap-1.5 mt-2">
					{#each emails as email}
						<span class="inline-flex items-center gap-1 badge badge-info text-xs">
							{email}
							<button type="button" onclick={() => removeEmail(email)} class="hover:text-destructive ml-0.5">
								<X class="w-3 h-3" />
							</button>
						</span>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Duration -->
		<div>
			<label class="block text-sm text-content-secondary mb-1"><Clock class="w-3.5 h-3.5 inline" /> Duración del enlace</label>
			<select bind:value={durationHours} class="input py-2 text-sm">
				{#each durationOptions as opt}
					<option value={opt.value}>{opt.label}</option>
				{/each}
			</select>
		</div>

		<!-- Schedule toggle -->
		<div>
			<label class="flex items-center gap-2 text-sm cursor-pointer">
				<input type="checkbox" bind:checked={scheduleEnabled}
					class="rounded bg-surface-raised border-edge text-blue-600 focus:ring-blue-500" />
				<Calendar class="w-3.5 h-3.5 text-content-muted" />
				<span class="text-content-secondary">Horario de disponibilidad</span>
			</label>
			{#if scheduleEnabled}
				<div class="flex items-center gap-2 mt-2 pl-6">
					<input type="time" bind:value={scheduleStart} class="input py-1.5 text-sm w-28" />
					<span class="text-content-muted text-xs">a</span>
					<input type="time" bind:value={scheduleEnd} class="input py-1.5 text-sm w-28" />
					<span class="text-xs text-content-muted">(horario UTC)</span>
				</div>
				<p class="text-[10px] text-content-muted pl-6 mt-1">Las cámaras solo se podrán ver dentro de este horario</p>
			{/if}
		</div>

		{#if error}
			<div class="flex items-center gap-2 text-sm rounded-lg px-3.5 py-2.5 border"
				style="background: var(--th-badge-danger-bg); color: var(--th-badge-danger-text); border-color: var(--th-badge-danger-bg);">
				<AlertCircle class="w-4 h-4 shrink-0" /><span>{error}</span>
			</div>
		{/if}

		<button type="button" onclick={handleCreate} class="btn btn-primary w-full py-2.5"
			disabled={emails.length === 0}>
			<Link class="w-4 h-4" /> Crear enlace compartido
		</button>

		<!-- Existing shares -->
		{#if mosaicShares.length > 0}
			<hr class="border-edge" />
			<div>
				<h4 class="text-sm font-medium text-content mb-2">Enlaces activos ({mosaicShares.length})</h4>
				<div class="space-y-2 max-h-48 overflow-y-auto">
					{#each mosaicShares as share (share.id)}
						<div class="bg-surface-raised border border-edge rounded-lg p-3 text-xs space-y-1.5 {share.active ? '' : 'opacity-50'}">
							<div class="flex items-center justify-between gap-2">
								<span class="text-content-muted truncate flex-1">{share.emails}</span>
								<div class="flex items-center gap-1 shrink-0">
									<button onclick={() => copyShareLink(share.token)}
										class="btn btn-ghost p-1" title="Copiar enlace">
										{#if copiedToken === share.token}
											<Check class="w-3.5 h-3.5 text-emerald-500" />
										{:else}
											<Copy class="w-3.5 h-3.5" />
										{/if}
									</button>
									<button onclick={() => app.toggleShare(share.id)}
										class="btn btn-ghost p-1" title={share.active ? 'Desactivar' : 'Activar'}>
										{#if share.active}<ToggleRight class="w-3.5 h-3.5 text-emerald-500" />{:else}<ToggleLeft class="w-3.5 h-3.5" />{/if}
									</button>
									<button onclick={() => app.deleteShare(share.id)}
										class="btn btn-ghost p-1 text-destructive" title="Eliminar">
										<Trash2 class="w-3.5 h-3.5" />
									</button>
								</div>
							</div>
							<div class="flex items-center gap-3 text-content-muted">
								<span>Expira: {new Date(share.expires_at).toLocaleString()}</span>
								{#if share.schedule_start && share.schedule_end}
									<span>{share.schedule_start} - {share.schedule_end}</span>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</Modal>
