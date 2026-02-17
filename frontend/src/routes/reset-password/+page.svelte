<script lang="ts">
	import { Video, ArrowLeft, KeyRound, CheckCircle2, AlertCircle, Lock, Eye, EyeOff } from 'lucide-svelte';
	import { brand } from '$lib/brand.config';
	import { page } from '$app/stores';
	import { z } from 'zod';

	const schema = z.object({
		password: z
			.string()
			.min(1, 'La contraseña es requerida')
			.min(6, 'La contraseña debe tener al menos 6 caracteres')
			.max(128, 'La contraseña no puede superar 128 caracteres'),
		confirmPassword: z.string().min(1, 'Confirma tu contraseña')
	}).refine((data) => data.password === data.confirmPassword, {
		message: 'Las contraseñas no coinciden',
		path: ['confirmPassword']
	});

	let password = $state('');
	let confirmPassword = $state('');
	let loading = $state(false);
	let success = $state(false);
	let error = $state('');
	let submitted = $state(false);
	let showPassword = $state(false);
	let showConfirm = $state(false);
	let touched = $state<Record<string, boolean>>({ password: false, confirmPassword: false });

	let token = $derived($page.url.searchParams.get('token') || '');

	let fieldErrors = $derived.by(() => {
		const result = schema.safeParse({ password, confirmPassword });
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

	// Password strength indicator
	let strength = $derived.by(() => {
		if (!password) return { level: 0, label: '', color: '' };
		let score = 0;
		if (password.length >= 6) score++;
		if (password.length >= 10) score++;
		if (/[A-Z]/.test(password)) score++;
		if (/[0-9]/.test(password)) score++;
		if (/[^A-Za-z0-9]/.test(password)) score++;
		if (score <= 1) return { level: 1, label: 'Débil', color: 'bg-red-500' };
		if (score <= 2) return { level: 2, label: 'Regular', color: 'bg-amber-500' };
		if (score <= 3) return { level: 3, label: 'Buena', color: 'bg-blue-500' };
		return { level: 4, label: 'Fuerte', color: 'bg-emerald-500' };
	});

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		submitted = true;
		error = '';
		if (!isValid) return;
		if (!token) { error = 'Token de restablecimiento no válido'; return; }

		loading = true;
		try {
			const res = await fetch('/auth/reset-password', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ token, password })
			});
			const json = await res.json();
			if (json.success) {
				success = true;
			} else {
				error = json.error || 'Error restableciendo contraseña';
			}
		} catch {
			error = 'Error de conexión con el servidor';
		} finally {
			loading = false;
		}
	}
</script>

<div class="min-h-screen flex items-center justify-center px-5 py-8">
	<div class="w-full max-w-[400px]">
		<div class="text-center mb-8">
			<div class="w-14 h-14 bg-primary rounded-2xl flex items-center justify-center mx-auto mb-3 shadow-lg">
				<Video class="w-7 h-7 text-white" />
			</div>
			<h1 class="text-xl font-bold text-content">{brand.name}</h1>
			<p class="text-sm text-content-muted mt-0.5">Nueva contraseña</p>
		</div>

		{#if !token}
			<div class="bg-surface-alt border border-edge rounded-xl p-6 text-center" style="box-shadow: 0 4px 24px var(--th-shadow);">
				<div class="w-12 h-12 bg-red-100 dark:bg-red-900/30 rounded-full flex items-center justify-center mx-auto mb-4">
					<AlertCircle class="w-6 h-6 text-red-600 dark:text-red-400" />
				</div>
				<p class="text-sm text-content-secondary mb-5">Token de restablecimiento no encontrado o inválido.</p>
				<a href="/forgot-password" class="btn btn-primary w-full py-2.5">Solicitar nuevo enlace</a>
			</div>
		{:else if success}
			<div class="bg-surface-alt border border-edge rounded-xl p-6 text-center" style="box-shadow: 0 4px 24px var(--th-shadow);">
				<div class="w-12 h-12 bg-emerald-100 dark:bg-emerald-900/30 rounded-full flex items-center justify-center mx-auto mb-4">
					<CheckCircle2 class="w-6 h-6 text-emerald-600 dark:text-emerald-400" />
				</div>
				<h2 class="text-lg font-semibold text-content mb-2">Contraseña actualizada</h2>
				<p class="text-sm text-content-secondary mb-5">Tu contraseña ha sido restablecida exitosamente.</p>
				<a href="/login" class="btn btn-primary w-full py-2.5">Iniciar sesión</a>
			</div>
		{:else}
			<form onsubmit={handleSubmit} class="space-y-5" novalidate>
				<p class="text-sm text-content-secondary">Crea tu nueva contraseña segura.</p>

				<!-- Password -->
				<div>
					<label for="rp-pass" class="block text-sm font-medium text-content-secondary mb-1.5">Nueva contraseña</label>
					<div class="relative">
						<Lock class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
						<input
							id="rp-pass"
							type={showPassword ? 'text' : 'password'}
							bind:value={password}
							onblur={() => (touched.password = true)}
							class="input py-2.5 pl-10 pr-16 {showErr('password') ? 'input-error' : touched.password && !fieldErrors.password ? 'input-success' : ''}"
							placeholder="Mínimo 6 caracteres"
						/>
						<div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
							{#if touched.password && !fieldErrors.password}
								<CheckCircle2 class="w-4 h-4 text-emerald-500" />
							{/if}
							{#if showErr('password')}
								<AlertCircle class="w-4 h-4 text-red-500" />
							{/if}
							<button type="button" onclick={() => (showPassword = !showPassword)} class="p-1 text-content-muted hover:text-content transition rounded" tabindex={-1}>
								{#if showPassword}<EyeOff class="w-4 h-4" />{:else}<Eye class="w-4 h-4" />{/if}
							</button>
						</div>
					</div>
					{#if showErr('password')}
						<p class="text-xs text-red-500 mt-1.5">{showErr('password')}</p>
					{/if}
					<!-- Strength meter -->
					{#if password.length > 0}
						<div class="mt-2">
							<div class="flex gap-1 h-1">
								{#each [1, 2, 3, 4] as bar}
									<div class="flex-1 rounded-full transition-colors {bar <= strength.level ? strength.color : 'bg-surface-raised'}"></div>
								{/each}
							</div>
							<p class="text-[10px] mt-1 text-content-muted">Seguridad: <span class="font-medium">{strength.label}</span></p>
						</div>
					{/if}
				</div>

				<!-- Confirm Password -->
				<div>
					<label for="rp-confirm" class="block text-sm font-medium text-content-secondary mb-1.5">Confirmar contraseña</label>
					<div class="relative">
						<Lock class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
						<input
							id="rp-confirm"
							type={showConfirm ? 'text' : 'password'}
							bind:value={confirmPassword}
							onblur={() => (touched.confirmPassword = true)}
							class="input py-2.5 pl-10 pr-16 {showErr('confirmPassword') ? 'input-error' : touched.confirmPassword && !fieldErrors.confirmPassword ? 'input-success' : ''}"
							placeholder="Repetir contraseña"
						/>
						<div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
							{#if touched.confirmPassword && !fieldErrors.confirmPassword}
								<CheckCircle2 class="w-4 h-4 text-emerald-500" />
							{/if}
							{#if showErr('confirmPassword')}
								<AlertCircle class="w-4 h-4 text-red-500" />
							{/if}
							<button type="button" onclick={() => (showConfirm = !showConfirm)} class="p-1 text-content-muted hover:text-content transition rounded" tabindex={-1}>
								{#if showConfirm}<EyeOff class="w-4 h-4" />{:else}<Eye class="w-4 h-4" />{/if}
							</button>
						</div>
					</div>
					{#if showErr('confirmPassword')}
						<p class="text-xs text-red-500 mt-1.5">{showErr('confirmPassword')}</p>
					{/if}
				</div>

				{#if error}
					<div class="flex items-center gap-2 text-sm rounded-lg px-3.5 py-2.5 border"
						style="background: var(--th-badge-danger-bg); color: var(--th-badge-danger-text); border-color: var(--th-badge-danger-bg);">
						<AlertCircle class="w-4 h-4 shrink-0" />
						<span>{error}</span>
					</div>
				{/if}

				<button type="submit" disabled={loading} class="btn btn-primary w-full py-3 text-sm font-semibold disabled:opacity-50 transition-all
					{loading ? '' : 'hover:shadow-lg hover:shadow-blue-500/25 active:scale-[0.98]'}">
					{#if loading}
						<span class="animate-spin inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full"></span>
						Guardando...
					{:else}
						<KeyRound class="w-4 h-4" /> Restablecer contraseña
					{/if}
				</button>
				<div class="text-center">
					<a href="/login" class="text-xs text-primary hover:underline inline-flex items-center gap-1 transition">
						<ArrowLeft class="w-3 h-3" /> Volver al login
					</a>
				</div>
			</form>
		{/if}
	</div>
</div>
