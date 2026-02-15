<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';
	import { login } from '$lib/api';
	import { Video, LogIn } from 'lucide-svelte';
	import { onMount } from 'svelte';

	let username = $state('');
	let password = $state('');
	let error = $state('');
	let loading = $state(false);

	onMount(() => {
		if (auth.isAuthenticated) goto('/');
	});

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		error = '';
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
</script>

<div class="min-h-screen flex items-center justify-center px-4">
	<div class="w-full max-w-sm">
		<div class="text-center mb-8">
			<div class="w-16 h-16 bg-blue-600 rounded-2xl flex items-center justify-center mx-auto mb-4">
				<Video class="w-8 h-8 text-white" />
			</div>
			<h1 class="text-2xl font-bold text-white">MediaMTX</h1>
			<p class="text-sm text-gray-500 mt-1">Camera Manager</p>
		</div>
		<form onsubmit={handleSubmit} class="bg-gray-900 border border-gray-800 rounded-xl p-6 space-y-4">
			<div>
				<label for="login-user" class="block text-xs text-gray-400 mb-1.5">Usuario</label>
				<input
					id="login-user"
					type="text"
					bind:value={username}
					required
					autocomplete="username"
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-sm focus:outline-none focus:border-blue-500 transition"
					placeholder="admin"
				/>
			</div>
			<div>
				<label for="login-pass" class="block text-xs text-gray-400 mb-1.5">Contraseña</label>
				<input
					id="login-pass"
					type="password"
					bind:value={password}
					required
					autocomplete="current-password"
					class="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2.5 text-sm focus:outline-none focus:border-blue-500 transition"
					placeholder="••••••"
				/>
			</div>
			{#if error}
				<div class="text-red-400 text-sm bg-red-900/20 border border-red-800 rounded-lg px-3 py-2">{error}</div>
			{/if}
			<button
				type="submit"
				disabled={loading}
				class="w-full py-2.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 rounded-lg text-sm font-medium transition flex items-center justify-center gap-2"
			>
				{#if loading}
					Autenticando...
				{:else}
					<LogIn class="w-4 h-4" /> Iniciar sesión
				{/if}
			</button>
		</form>
	</div>
</div>
