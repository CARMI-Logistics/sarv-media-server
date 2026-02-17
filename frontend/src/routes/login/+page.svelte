<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { login } from '$lib/api';
	import {
		Video, LogIn, Shield, Smartphone, Wifi, LayoutDashboard,
		Eye, EyeOff, User, Lock, CheckCircle2, AlertCircle
	} from 'lucide-svelte';
	import { onMount } from 'svelte';
	import { brand } from '$lib/brand.config';
	import { z } from 'zod';

	// ── Zod schema ──
	const loginSchema = z.object({
		username: z
			.string()
			.min(1, 'El usuario es requerido')
			.min(3, 'El usuario debe tener al menos 3 caracteres')
			.max(50, 'El usuario no puede superar 50 caracteres'),
		password: z
			.string()
			.min(1, 'La contraseña es requerida')
			.min(4, 'La contraseña debe tener al menos 4 caracteres')
	});

	// ── State ──
	let username = $state('');
	let password = $state('');
	let error = $state('');
	let loading = $state(false);
	let showPassword = $state(false);
	let submitted = $state(false);

	// Per-field touched state (show errors after blur or submit)
	let touched = $state<Record<string, boolean>>({ username: false, password: false });

	// ── Derived per-field errors via Zod ──
	let fieldErrors = $derived.by(() => {
		const result = loginSchema.safeParse({ username, password });
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

	function handleBlur(field: string) {
		touched[field] = true;
	}

	onMount(() => {
		if (auth.isAuthenticated) goto('/');
	});

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		submitted = true;
		error = '';

		if (!isValid) return;

		loading = true;
		try {
			const result = await login(username.trim(), password);
			if (result.token) {
				auth.setToken(result.token);
				goto('/');
			} else {
				error = result.error || 'Credenciales inválidas';
			}
		} finally {
			loading = false;
		}
	}

	// Features showcase data
	const features = [
		{ icon: Video, title: 'Monitoreo en Vivo', desc: 'Streaming WebRTC y HLS en tiempo real' },
		{ icon: LayoutDashboard, title: 'Mosaicos Dinámicos', desc: 'Visualiza múltiples cámaras simultáneamente' },
		{ icon: Shield, title: 'Seguridad Empresarial', desc: 'Autenticación JWT con cifrado RS256' },
		{ icon: Smartphone, title: 'Multiplataforma', desc: 'Compatible con desktop, tablet y móvil' },
		{ icon: Wifi, title: 'Multi-protocolo', desc: 'Soporte RTSP, RTMP, HLS, WebRTC y SRT' },
	];
</script>

<div class="min-h-screen flex flex-col lg:flex-row">
	<!-- ═══════════ LEFT PANEL — Features Showcase ═══════════ -->
	<div class="hidden lg:flex lg:w-1/2 relative overflow-hidden flex-col justify-between p-10 xl:p-14"
		style="background: linear-gradient(135deg, #1e3a5f 0%, #0f172a 50%, #1a1a2e 100%);">
		<!-- Decorative elements -->
		<div class="absolute top-0 right-0 w-96 h-96 rounded-full opacity-[0.07]" style="background: radial-gradient(circle, #3b82f6, transparent 70%); transform: translate(30%, -30%);"></div>
		<div class="absolute bottom-0 left-0 w-72 h-72 rounded-full opacity-[0.05]" style="background: radial-gradient(circle, #60a5fa, transparent 70%); transform: translate(-30%, 30%);"></div>

		<!-- Header -->
		<div class="relative z-10">
			<div class="flex items-center gap-3 mb-2">
				<div class="w-11 h-11 bg-blue-500/20 backdrop-blur rounded-xl flex items-center justify-center border border-blue-400/20">
					<Video class="w-6 h-6 text-blue-400" />
				</div>
				<div>
					<h1 class="text-xl font-bold text-white">{brand.name}</h1>
					<p class="text-xs text-blue-300/70">{brand.tagline}</p>
				</div>
			</div>
		</div>

		<!-- Features list -->
		<div class="relative z-10 space-y-5 my-auto">
			<h2 class="text-2xl xl:text-3xl font-bold text-white leading-tight">
				Gestión inteligente de<br/>videovigilancia
			</h2>
			<p class="text-sm text-slate-400 max-w-md leading-relaxed">
				Plataforma centralizada para monitorear, grabar y gestionar cámaras de seguridad con tecnología de última generación.
			</p>
			<div class="space-y-3.5 pt-2">
				{#each features as feat}
					{@const Icon = feat.icon}
					<div class="flex items-start gap-3.5 group">
						<div class="w-9 h-9 rounded-lg bg-white/[0.06] border border-white/[0.08] flex items-center justify-center shrink-0 group-hover:bg-blue-500/15 group-hover:border-blue-400/20 transition">
							<Icon class="w-4.5 h-4.5 text-blue-400" />
						</div>
						<div>
							<p class="text-sm font-medium text-white">{feat.title}</p>
							<p class="text-xs text-slate-400 mt-0.5">{feat.desc}</p>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<!-- Footer -->
		<div class="relative z-10 flex items-center justify-between text-xs text-slate-500">
			<span>&copy; {new Date().getFullYear()} {brand.name}</span>
			<span class="flex items-center gap-1.5">
				<span class="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_6px_rgba(34,197,94,0.6)]"></span>
				Sistema operativo
			</span>
		</div>
	</div>

	<!-- ═══════════ RIGHT PANEL — Login Form ═══════════ -->
	<div class="flex-1 flex items-center justify-center px-5 py-8 sm:px-8">
		<div class="w-full max-w-[400px]">
			<!-- Mobile-only header -->
			<div class="text-center mb-8 lg:hidden">
				<div class="w-14 h-14 bg-primary rounded-2xl flex items-center justify-center mx-auto mb-3 shadow-lg">
					<Video class="w-7 h-7 text-white" />
				</div>
				<h1 class="text-xl font-bold text-content">{brand.name}</h1>
				<p class="text-sm text-content-muted mt-0.5">{brand.tagline}</p>
			</div>

			<!-- Desktop heading -->
			<div class="hidden lg:block mb-8">
				<h2 class="text-2xl font-bold text-content">Iniciar sesión</h2>
				<p class="text-sm text-content-muted mt-1">Ingresa tus credenciales para acceder al panel</p>
			</div>

			<!-- Form -->
			<form onsubmit={handleSubmit} class="space-y-5" novalidate>
				<!-- Username -->
				<div>
					<label for="login-user" class="block text-sm font-medium text-content-secondary mb-1.5">Usuario</label>
					<div class="relative">
						<User class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
						<input
							id="login-user"
							type="text"
							bind:value={username}
							onblur={() => handleBlur('username')}
							autocomplete="username"
							class="input py-2.5 pl-10 pr-9 {showError('username') ? 'input-error' : touched.username && !fieldErrors.username ? 'input-success' : ''}"
							placeholder="Tu nombre de usuario"
						/>
						{#if touched.username && !fieldErrors.username}
							<CheckCircle2 class="w-4 h-4 absolute right-3 top-1/2 -translate-y-1/2 text-emerald-500" />
						{/if}
						{#if showError('username')}
							<AlertCircle class="w-4 h-4 absolute right-3 top-1/2 -translate-y-1/2 text-red-500" />
						{/if}
					</div>
					{#if showError('username')}
						<p class="text-xs text-red-500 mt-1.5 flex items-center gap-1">
							{showError('username')}
						</p>
					{/if}
				</div>

				<!-- Password -->
				<div>
					<label for="login-pass" class="block text-sm font-medium text-content-secondary mb-1.5">Contraseña</label>
					<div class="relative">
						<Lock class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
						<input
							id="login-pass"
							type={showPassword ? 'text' : 'password'}
							bind:value={password}
							onblur={() => handleBlur('password')}
							autocomplete="current-password"
							class="input py-2.5 pl-10 pr-20 {showError('password') ? 'input-error' : touched.password && !fieldErrors.password ? 'input-success' : ''}"
							placeholder="Tu contraseña"
						/>
						<div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
							{#if touched.password && !fieldErrors.password}
								<CheckCircle2 class="w-4 h-4 text-emerald-500" />
							{/if}
							{#if showError('password')}
								<AlertCircle class="w-4 h-4 text-red-500" />
							{/if}
							<button
								type="button"
								onclick={() => (showPassword = !showPassword)}
								class="p-1 text-content-muted hover:text-content transition rounded"
								tabindex={-1}
							>
								{#if showPassword}<EyeOff class="w-4 h-4" />{:else}<Eye class="w-4 h-4" />{/if}
							</button>
						</div>
					</div>
					{#if showError('password')}
						<p class="text-xs text-red-500 mt-1.5">
							{showError('password')}
						</p>
					{/if}
				</div>

				<!-- Forgot password link -->
				{#if brand.enablePasswordReset}
					<div class="flex justify-end -mt-1">
						<a href="/forgot-password" class="text-xs text-primary hover:underline transition">
							¿Olvidaste tu contraseña?
						</a>
					</div>
				{/if}

				<!-- Server error -->
				{#if error}
					<div class="flex items-center gap-2 text-sm rounded-lg px-3.5 py-2.5 border"
						style="background: var(--th-badge-danger-bg); color: var(--th-badge-danger-text); border-color: var(--th-badge-danger-bg);">
						<AlertCircle class="w-4 h-4 shrink-0" />
						<span>{error}</span>
					</div>
				{/if}

				<!-- Submit -->
				<button
					type="submit"
					disabled={loading}
					class="btn btn-primary w-full py-3 text-sm font-semibold disabled:opacity-50 transition-all
						{loading ? '' : 'hover:shadow-lg hover:shadow-blue-500/25 active:scale-[0.98]'}"
				>
					{#if loading}
						<span class="animate-spin inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full"></span>
						Autenticando...
					{:else}
						<LogIn class="w-4 h-4" /> Iniciar sesión
					{/if}
				</button>
			</form>

			<!-- Mobile-only features summary -->
			<div class="lg:hidden mt-8 pt-6 border-t border-edge">
				<div class="grid grid-cols-2 gap-3">
					{#each features.slice(0, 4) as feat}
						{@const Icon = feat.icon}
						<div class="flex items-center gap-2 text-xs text-content-muted">
							<Icon class="w-3.5 h-3.5 text-primary shrink-0" />
							<span>{feat.title}</span>
						</div>
					{/each}
				</div>
			</div>
		</div>
	</div>
</div>
