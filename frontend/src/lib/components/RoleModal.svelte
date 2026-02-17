<script lang="ts">
	import Modal from './Modal.svelte';
	import { app } from '$lib/stores/app.svelte';
	import type { RoleWithPermissions, PermissionInput } from '$lib/types';
	import { z } from 'zod';
	import { AlertCircle } from 'lucide-svelte';

	let {
		open = $bindable(false),
		editRole = null as RoleWithPermissions | null
	}: {
		open: boolean;
		editRole: RoleWithPermissions | null;
	} = $props();

	const schema = z.object({
		name: z.string().min(1, 'El nombre es requerido').min(2, 'Mínimo 2 caracteres').max(50, 'Máximo 50 caracteres'),
		description: z.string().max(200, 'Máximo 200 caracteres')
	});

	const modules = ['cameras', 'mosaics', 'locations', 'users', 'captures', 'notifications', 'roles', 'settings'];
	const moduleLabels: Record<string, string> = {
		cameras: 'Cámaras', mosaics: 'Mosaicos', locations: 'Ubicaciones', users: 'Usuarios',
		captures: 'Capturas', notifications: 'Notificaciones', roles: 'Roles', settings: 'Ajustes'
	};

	let name = $state('');
	let description = $state('');
	let permissions = $state<Record<string, { can_view: boolean; can_create: boolean; can_edit: boolean; can_delete: boolean }>>({});
	let error = $state('');
	let submitted = $state(false);
	let touched = $state<Record<string, boolean>>({});

	let fieldErrors = $derived.by(() => {
		const result = schema.safeParse({ name, description });
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
			if (editRole) {
				name = editRole.name;
				description = editRole.description || '';
				const perms: typeof permissions = {};
				for (const m of modules) {
					const p = editRole.permissions.find((p) => p.module === m);
					perms[m] = p ? { can_view: p.can_view, can_create: p.can_create, can_edit: p.can_edit, can_delete: p.can_delete } : { can_view: false, can_create: false, can_edit: false, can_delete: false };
				}
				permissions = perms;
			} else {
				name = '';
				description = '';
				const perms: typeof permissions = {};
				for (const m of modules) perms[m] = { can_view: false, can_create: false, can_edit: false, can_delete: false };
				permissions = perms;
			}
		}
	});

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		submitted = true;
		error = '';
		if (!isValid) return;

		const permList: PermissionInput[] = modules.map((m) => ({
			module: m,
			...permissions[m]
		}));

		const body = { name: name.trim(), description: description.trim(), permissions: permList };
		const ok = await app.saveRole(editRole?.id ?? null, body);
		if (ok) open = false;
		else error = 'Error guardando rol';
	}

	function toggleAll(mod: string, checked: boolean) {
		permissions[mod] = { can_view: checked, can_create: checked, can_edit: checked, can_delete: checked };
	}
</script>

<Modal bind:open title={editRole ? 'Editar Rol' : 'Nuevo Rol'} maxWidth="max-w-2xl">
	<form onsubmit={handleSubmit} class="space-y-4" novalidate>
		<div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
			<div>
				<label for="role-name" class="block text-sm text-content-secondary mb-1">Nombre *</label>
				<input id="role-name" type="text" bind:value={name} onblur={() => (touched.name = true)}
					class="input {showErr('name') ? 'input-error' : ''}" placeholder="ej: supervisor"
					disabled={editRole?.is_system} />
				{#if showErr('name')}
					<p class="text-xs text-red-500 mt-1 flex items-center gap-1"><AlertCircle class="w-3 h-3" />{showErr('name')}</p>
				{/if}
			</div>
			<div>
				<label for="role-desc" class="block text-sm text-content-secondary mb-1">Descripción</label>
				<input id="role-desc" type="text" bind:value={description} onblur={() => (touched.description = true)}
					class="input {showErr('description') ? 'input-error' : ''}" placeholder="Descripción del rol" />
				{#if showErr('description')}
					<p class="text-xs text-red-500 mt-1">{showErr('description')}</p>
				{/if}
			</div>
		</div>

		<div>
			<h4 class="text-sm font-medium text-content mb-2">Permisos por módulo</h4>
			<div class="border border-edge rounded-lg overflow-hidden">
				<div class="hidden sm:grid grid-cols-6 gap-0 bg-surface-raised text-xs text-content-muted font-medium">
					<div class="col-span-2 px-3 py-2">Módulo</div>
					<div class="px-3 py-2 text-center">Ver</div>
					<div class="px-3 py-2 text-center">Crear</div>
					<div class="px-3 py-2 text-center">Editar</div>
					<div class="px-3 py-2 text-center">Eliminar</div>
				</div>
				{#each modules as mod}
					<div class="grid grid-cols-2 sm:grid-cols-6 gap-0 border-t border-edge items-center">
						<div class="col-span-2 px-3 py-2 flex items-center justify-between">
							<span class="text-sm text-content">{moduleLabels[mod] || mod}</span>
							<button type="button" onclick={() => toggleAll(mod, !permissions[mod]?.can_view)}
								class="text-[10px] text-primary hover:underline sm:hidden">Toggle</button>
						</div>
						{#each ['can_view', 'can_create', 'can_edit', 'can_delete'] as perm}
							<div class="px-3 py-2 flex items-center justify-center">
								<label class="flex items-center gap-1.5 sm:gap-0 cursor-pointer">
									<span class="text-[10px] text-content-muted sm:hidden capitalize">{perm.replace('can_', '')}</span>
									<input type="checkbox" bind:checked={permissions[mod][perm as keyof typeof permissions[typeof mod]]}
										class="rounded bg-surface-raised border-edge text-blue-600 focus:ring-blue-500" />
								</label>
							</div>
						{/each}
					</div>
				{/each}
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
