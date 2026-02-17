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
		{ icon: Video, title: 'Streaming en Tiempo Real', desc: 'Monitoreo continuo con WebRTC y HLS de baja latencia' },
		{ icon: LayoutDashboard, title: 'Mosaicos Personalizables', desc: 'Crea vistas personalizadas con múltiples cámaras' },
		{ icon: Shield, title: 'Seguridad de Nivel Empresarial', desc: 'Autenticación robusta y control de acceso basado en roles' },
		{ icon: Smartphone, title: 'Acceso Multiplataforma', desc: 'Diseño responsive para cualquier dispositivo' },
		{ icon: Wifi, title: 'Protocolos Múltiples', desc: 'Compatible con RTSP, RTMP, HLS, WebRTC y SRT' },
	];
</script>

<div class="min-h-screen flex flex-col lg:flex-row">
	<!-- ═══════════ LEFT PANEL — Features Showcase ═══════════ -->
	<div class="hidden lg:flex lg:w-[55%] relative overflow-hidden flex-col justify-between p-12 xl:p-16"
		style="background: linear-gradient(135deg, #0f172a 0%, #1e293b 40%, #334155 100%);">
		<!-- Decorative grid pattern -->
		<div class="absolute inset-0 opacity-[0.03]" style="background-image: radial-gradient(circle at 1px 1px, rgb(255 255 255) 1px, transparent 0); background-size: 40px 40px;"></div>
		<!-- Decorative elements -->
		<div class="absolute top-0 right-0 w-[500px] h-[500px] rounded-full opacity-[0.08]" style="background: radial-gradient(circle, #3b82f6, transparent 70%); transform: translate(40%, -40%);"></div>
		<div class="absolute bottom-0 left-0 w-[400px] h-[400px] rounded-full opacity-[0.06]" style="background: radial-gradient(circle, #60a5fa, transparent 70%); transform: translate(-40%, 40%);"></div>

		<!-- Header -->
		<div class="relative z-10">
			<div class="flex items-center gap-3.5 mb-3">
				<div class="w-12 h-12 bg-gradient-to-br from-blue-500 to-blue-600 rounded-2xl flex items-center justify-center shadow-lg shadow-blue-500/25">
					<Video class="w-6 h-6 text-white" />
				</div>
				<div>
					<h1 class="text-xl font-bold text-white tracking-tight">{brand.name}</h1>
					<p class="text-xs text-blue-300/80 font-medium">{brand.tagline}</p>
				</div>
			</div>
		</div>

		<!-- Features list -->
		<div class="relative z-10 space-y-6 my-auto">
			<div class="space-y-3">
				<h2 class="text-3xl xl:text-4xl font-bold text-white leading-tight tracking-tight">
					Plataforma Integral de<br/>Videovigilancia
				</h2>
				<p class="text-base text-slate-300 max-w-lg leading-relaxed">
					Solución centralizada para monitorear, grabar y gestionar sistemas de cámaras de seguridad con tecnología de vanguardia.
				</p>
			</div>
			<div class="space-y-4 pt-4">
				{#each features as feat}
					{@const Icon = feat.icon}
					<div class="flex items-start gap-4 group cursor-default">
						<div class="w-10 h-10 rounded-xl bg-white/[0.06] border border-white/[0.08] flex items-center justify-center shrink-0 group-hover:bg-blue-500/15 group-hover:border-blue-400/25 transition-all duration-200">
							<Icon class="w-5 h-5 text-blue-400" />
						</div>
						<div class="flex-1">
							<p class="text-sm font-semibold text-white mb-0.5">{feat.title}</p>
							<p class="text-sm text-slate-400 leading-relaxed">{feat.desc}</p>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<!-- Footer -->
		<div class="relative z-10 flex items-center justify-between text-xs text-slate-400">
			<span class="font-medium">&copy; {new Date().getFullYear()} {brand.name}. Todos los derechos reservados.</span>
			<span class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-emerald-500/10 border border-emerald-500/20">
				<span class="w-2 h-2 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(34,197,94,0.6)] animate-pulse"></span>
				<span class="text-emerald-400 font-semibold">Sistema Activo</span>
			</span>
		</div>
	</div>

	<!-- ═══════════ RIGHT PANEL — Login Form ═══════════ -->
	<div class="flex-1 flex items-center justify-center px-6 py-10 sm:px-10 lg:px-16">
		<div class="w-full max-w-[440px]">
			<!-- Mobile-only header -->
			<div class="text-center mb-10 lg:hidden">
				<div class="w-16 h-16 bg-gradient-to-br from-blue-500 to-blue-600 rounded-2xl flex items-center justify-center mx-auto mb-4 shadow-xl shadow-blue-500/25">
					<Video class="w-8 h-8 text-white" />
				</div>
				<h1 class="text-2xl font-bold text-content tracking-tight">{brand.name}</h1>
				<p class="text-sm text-content-muted mt-1">{brand.tagline}</p>
			</div>

			<!-- Desktop heading -->
			<div class="hidden lg:block mb-10">
				<h2 class="text-3xl font-bold text-content tracking-tight">Bienvenido</h2>
				<p class="text-base text-content-muted mt-2">Ingresa tus credenciales para acceder a la plataforma</p>
			</div>

			<!-- Form -->
			<form onsubmit={handleSubmit} class="space-y-5" novalidate>
				<!-- Username -->
				<div>
					<label for="login-user" class="block text-sm font-semibold text-content mb-2">Usuario</label>
					<div class="relative">
						<User class="w-[18px] h-[18px] absolute left-4 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
						<input
							id="login-user"
							type="text"
							bind:value={username}
							onblur={() => handleBlur('username')}
							autocomplete="username"
							class="input py-3 pl-12 pr-11 text-[15px] {showError('username') ? 'input-error' : touched.username && !fieldErrors.username ? 'input-success' : ''}"
							placeholder="Ingresa tu usuario"
						/>
						{#if touched.username && !fieldErrors.username}
							<CheckCircle2 class="w-[18px] h-[18px] absolute right-4 top-1/2 -translate-y-1/2 text-emerald-500" />
						{/if}
						{#if showError('username')}
							<AlertCircle class="w-[18px] h-[18px] absolute right-4 top-1/2 -translate-y-1/2 text-red-500" />
						{/if}
					</div>
					{#if showError('username')}
						<p class="text-xs text-red-500 mt-2 flex items-center gap-1.5 font-medium">
							{showError('username')}
						</p>
					{/if}
				</div>

				<!-- Password -->
				<div>
					<label for="login-pass" class="block text-sm font-semibold text-content mb-2">Contraseña</label>
					<div class="relative">
						<Lock class="w-[18px] h-[18px] absolute left-4 top-1/2 -translate-y-1/2 text-content-muted pointer-events-none" />
						<input
							id="login-pass"
							type={showPassword ? 'text' : 'password'}
							bind:value={password}
							onblur={() => handleBlur('password')}
							autocomplete="current-password"
							class="input py-3 pl-12 pr-24 text-[15px] {showError('password') ? 'input-error' : touched.password && !fieldErrors.password ? 'input-success' : ''}"
							placeholder="Ingresa tu contraseña"
						/>
						<div class="absolute right-3 top-1/2 -translate-y-1/2 flex items-center gap-1.5">
							{#if touched.password && !fieldErrors.password}
								<CheckCircle2 class="w-[18px] h-[18px] text-emerald-500" />
							{/if}
							{#if showError('password')}
								<AlertCircle class="w-[18px] h-[18px] text-red-500" />
							{/if}
							<button
								type="button"
								onclick={() => (showPassword = !showPassword)}
								class="p-1.5 text-content-muted hover:text-content transition-colors rounded-md hover:bg-surface-raised"
								tabindex={-1}
							>
								{#if showPassword}<EyeOff class="w-[18px] h-[18px]" />{:else}<Eye class="w-[18px] h-[18px]" />{/if}
							</button>
						</div>
					</div>
					{#if showError('password')}
						<p class="text-xs text-red-500 mt-2 font-medium">
							{showError('password')}
						</p>
					{/if}
				</div>

				<!-- Forgot password link -->
				{#if brand.enablePasswordReset}
					<div class="flex justify-end">
						<a href="/forgot-password" class="text-sm font-medium text-primary hover:text-primary/80 transition-colors">
							¿Olvidaste tu contraseña?
						</a>
					</div>
				{/if}

				<!-- Server error -->
				{#if error}
					<div class="flex items-center gap-3 text-sm rounded-xl px-4 py-3.5 border-2"
						style="background: var(--th-badge-danger-bg); color: var(--th-badge-danger-text); border-color: rgba(239, 68, 68, 0.3);">
						<AlertCircle class="w-5 h-5 shrink-0" />
						<span class="font-medium">{error}</span>
					</div>
				{/if}

				<!-- Submit -->
				<button
					type="submit"
					disabled={loading}
					class="btn btn-primary w-full py-3.5 text-[15px] font-semibold disabled:opacity-50 transition-all rounded-xl
						{loading ? '' : 'hover:shadow-xl hover:shadow-blue-500/25 active:scale-[0.98] hover:-translate-y-0.5'}"
				>
					{#if loading}
						<span class="animate-spin inline-block w-5 h-5 border-2 border-white/30 border-t-white rounded-full"></span>
						<span>Autenticando...</span>
					{:else}
						<LogIn class="w-5 h-5" />
						<span>Iniciar sesión</span>
					{/if}
				</button>
			</form>

			<!-- Mobile-only features summary -->
			<div class="lg:hidden mt-10 pt-8 border-t border-edge">
				<p class="text-xs font-semibold text-content-muted uppercase tracking-wider mb-4">Características principales</p>
				<div class="grid grid-cols-2 gap-4">
					{#each features.slice(0, 4) as feat}
						{@const Icon = feat.icon}
						<div class="flex items-center gap-2.5 text-sm text-content">
							<div class="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
								<Icon class="w-4 h-4 text-primary" />
							</div>
							<span class="font-medium">{feat.title}</span>
						</div>
					{/each}
				</div>
			</div>
		</div>
	</div>
</div>
