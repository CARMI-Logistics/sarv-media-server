<script lang="ts">
	import { Video, ArrowLeft, Mail, AlertCircle, CheckCircle2 } from 'lucide-svelte';
	import { brand } from '$lib/brand.config';
	import { z } from 'zod';

	const schema = z.object({
		email: z.string().min(1, 'El email es requerido').email('Ingresa un email válido')
	});

	let email = $state('');
	let loading = $state(false);
	let sent = $state(false);
	let error = $state('');
	let submitted = $state(false);
	let touched = $state<Record<string, boolean>>({ email: false });

	let fieldErrors = $derived.by(() => {
		const result = schema.safeParse({ email });
		if (result.success) return {} as Record<string, string>;
		const errs: Record<string, string> = {};
		for (const issue of result.error.issues) {
			const key = String(issue.path[0]);
			if (!errs[key]) errs[key] = issue.message;
		}
		return errs;
	});

	let isValid = $derived(Object.keys(fieldErrors).length === 0);

	function showError(field: string): string {
		if ((touched[field] || submitted) && fieldErrors[field]) return fieldErrors[field];
		return '';
	}

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		submitted = true;
		error = '';
		if (!isValid) return;

		loading = true;
		try {
			const res = await fetch('/auth/forgot-password', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ email })
			});
			const json = await res.json();
			if (json.success) {
				sent = true;
			} else {
				error = json.error || 'Error enviando email';
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
			<p class="text-sm text-content-muted mt-0.5">Restablecer contraseña</p>
		</div>

		{#if sent}
			<div class="bg-surface-alt border border-edge rounded-xl p-6 text-center" style="box-shadow: 0 4px 24px var(--th-shadow);">
				<div class="w-12 h-12 bg-emerald-100 dark:bg-emerald-900/30 rounded-full flex items-center justify-center mx-auto mb-4">
					<Mail class="w-6 h-6 text-emerald-600 dark:text-emerald-400" />
				</div>
				<h2 class="text-lg font-semibold text-content mb-2">Revisa tu email</h2>
				<p class="text-sm text-content-secondary mb-5">
					Si existe una cuenta con <strong>{email}</strong>, recibirás un enlace para restablecer tu contraseña.
				</p>
				<a href="/login" class="btn btn-primary w-full py-2.5">
					<ArrowLeft class="w-4 h-4" /> Volver al login
				</a>
			</div>
		{:else}
			<form onsubmit={handleSubmit} class="space-y-5" novalidate>
				<p class="text-sm text-content-secondary">
					Ingresa tu email y te enviaremos un enlace para restablecer tu contraseña.
				</p>
				<div>
					<label for="fp-email" class="block text-sm font-medium text-content-secondary mb-1.5">Email</label>
					<div class="relative">
						<Mail class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
						<input
							id="fp-email"
							type="email"
							bind:value={email}
							onblur={() => (touched.email = true)}
							class="input py-2.5 pl-10 pr-9 {showError('email') ? 'input-error' : touched.email && !fieldErrors.email ? 'input-success' : ''}"
							placeholder="tu@email.com"
						/>
						{#if touched.email && !fieldErrors.email}
							<CheckCircle2 class="w-4 h-4 absolute right-3 top-1/2 -translate-y-1/2 text-emerald-500" />
						{/if}
						{#if showError('email')}
							<AlertCircle class="w-4 h-4 absolute right-3 top-1/2 -translate-y-1/2 text-red-500" />
						{/if}
					</div>
					{#if showError('email')}
						<p class="text-xs text-red-500 mt-1.5">{showError('email')}</p>
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
						Enviando...
					{:else}
						<Mail class="w-4 h-4" /> Enviar enlace
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
