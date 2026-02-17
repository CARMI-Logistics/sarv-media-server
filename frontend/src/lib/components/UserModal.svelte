<script lang="ts">
	import Modal from '$lib/components/Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { UserPublic } from '$lib/types';
	import { z } from 'zod';
	import { AlertCircle } from 'lucide-svelte';

	let {
		open = $bindable(false),
		editUser = null as UserPublic | null
	}: {
		open: boolean;
		editUser: UserPublic | null;
	} = $props();

	let username = $state('');
	let email = $state('');
	let password = $state('');
	let role = $state('viewer');
	let active = $state(true);
	let error = $state('');
	let submitted = $state(false);
	let touched = $state<Record<string, boolean>>({});

	let schema = $derived(z.object({
		username: z.string().min(1, 'El usuario es requerido').min(3, 'Mínimo 3 caracteres').max(50, 'Máximo 50 caracteres'),
		email: z.string().min(1, 'El email es requerido').email('Email inválido'),
		password: editUser
			? z.string().refine((v) => v === '' || v.length >= 6, { message: 'Mínimo 6 caracteres si se cambia' })
			: z.string().min(1, 'La contraseña es requerida').min(6, 'Mínimo 6 caracteres'),
	}));

	let fieldErrors = $derived.by(() => {
		const result = schema.safeParse({ username, email, password });
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
			if (editUser) {
				username = editUser.username;
				email = editUser.email;
				password = '';
				role = editUser.role;
				active = editUser.active;
			} else {
				username = '';
				email = '';
				password = '';
				role = 'viewer';
				active = true;
			}
		}
	});

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		submitted = true;
		error = '';
		if (!isValid) return;

		const body: Record<string, unknown> = { username, email, role, active };
		if (password) body.password = password;

		const ok = await app.saveUser(editUser?.id ?? null, body);
		if (ok) open = false;
		else error = 'Error guardando usuario';
	}
</script>

<Modal bind:open title={editUser ? 'Editar Usuario' : 'Nuevo Usuario'} maxWidth="max-w-md">
	<form onsubmit={handleSubmit} class="space-y-4" novalidate>
		<div>
			<label for="user-name" class="block text-sm text-content-secondary mb-1">Nombre de usuario *</label>
			<input id="user-name" type="text" bind:value={username} onblur={() => (touched.username = true)}
				class="input {showErr('username') ? 'input-error' : ''}" placeholder="johndoe" />
			{#if showErr('username')}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('username')}</p>{/if}
		</div>
		<div>
			<label for="user-email" class="block text-sm text-content-secondary mb-1">Email *</label>
			<input id="user-email" type="email" bind:value={email} onblur={() => (touched.email = true)}
				class="input {showErr('email') ? 'input-error' : ''}" placeholder="john@example.com" />
			{#if showErr('email')}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('email')}</p>{/if}
		</div>
		<div>
			<label for="user-pass" class="block text-sm text-content-secondary mb-1">
				Contraseña {editUser ? '(dejar vacío para no cambiar)' : '*'}
			</label>
			<input id="user-pass" type="password" bind:value={password} onblur={() => (touched.password = true)}
				class="input {showErr('password') ? 'input-error' : ''}" placeholder="Mínimo 6 caracteres" />
			{#if showErr('password')}<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('password')}</p>{/if}
		</div>
		<div>
			<label for="user-role" class="block text-sm text-content-secondary mb-1">Rol</label>
			<select id="user-role" bind:value={role} class="input">
				{#each app.roles as r}
					<option value={r.name}>{r.name.charAt(0).toUpperCase() + r.name.slice(1)}</option>
				{/each}
			</select>
		</div>
		{#if editUser}
			<div>
				<label class="flex items-center gap-2 text-sm text-content-secondary cursor-pointer">
					<input type="checkbox" bind:checked={active} class="rounded bg-surface-raised border-edge text-blue-600 focus:ring-blue-500" />
					Cuenta activa
				</label>
			</div>
		{/if}
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
