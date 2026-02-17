<script lang="ts">
	import { app } from '$lib/stores/app.svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { theme } from '$lib/stores/theme.svelte';
	import { brand } from '$lib/brand.config';
	import type { Tab } from '$lib/types';
	import {
		Video, Grid3x3, MapPin, Users, Sun, Moon, LogOut, RefreshCw,
		ChevronLeft, ChevronRight, PanelLeftClose, PanelLeft, X,
		Camera, Bell, Shield
	} from 'lucide-svelte';

	let {
		collapsed = $bindable(false),
		mobileOpen = $bindable(false),
	}: {
		collapsed: boolean;
		mobileOpen: boolean;
	} = $props();

	const navItems: { id: Tab; label: string; icon: typeof Video; count: () => number | string }[] = [
		{ id: 'cameras', label: 'Cámaras', icon: Video, count: () => app.cameras.length },
		{ id: 'mosaics', label: 'Mosaicos', icon: Grid3x3, count: () => app.mosaics.length },
		{ id: 'locations', label: 'Ubicaciones', icon: MapPin, count: () => app.locations.length },
		{ id: 'captures', label: 'Capturas', icon: Camera, count: () => app.captures.length },
		{ id: 'users', label: 'Usuarios', icon: Users, count: () => app.users.length },
		{ id: 'notifications', label: 'Notificaciones', icon: Bell, count: () => app.unreadCount > 0 ? `${app.unreadCount}` : '0' },
		{ id: 'roles', label: 'Roles', icon: Shield, count: () => app.roles.length },
	];

	function navigate(tab: Tab) {
		app.activeTab = tab;
		mobileOpen = false;
	}
</script>

<!-- Mobile overlay -->
{#if mobileOpen}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-40 bg-black/50 backdrop-blur-sm lg:hidden" onclick={() => (mobileOpen = false)}></div>
{/if}

<!-- Sidebar -->
<aside
	class="fixed top-0 left-0 z-50 h-full flex flex-col bg-surface-alt border-r border-edge transition-all duration-200 ease-in-out
		{mobileOpen ? 'translate-x-0' : '-translate-x-full'}
		lg:translate-x-0 lg:static lg:z-auto
		{collapsed ? 'w-[68px]' : 'w-60'}"
	style="box-shadow: 2px 0 8px var(--th-shadow);"
>
	<!-- Header -->
	<div class="flex items-center gap-3 px-4 py-4 border-b border-edge shrink-0 {collapsed ? 'justify-center' : ''}">
		{#if !collapsed}
			<div class="w-8 h-8 bg-primary rounded-lg flex items-center justify-center shrink-0">
				<Video class="w-4 h-4 text-white" />
			</div>
			<div class="min-w-0 flex-1">
				<h1 class="text-sm font-bold text-content truncate">{brand.name}</h1>
				<p class="text-[10px] text-content-muted truncate">{brand.tagline}</p>
			</div>
			<button onclick={() => (mobileOpen = false)} class="lg:hidden p-1 text-content-muted hover:text-content rounded">
				<X class="w-4 h-4" />
			</button>
		{:else}
			<div class="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
				<Video class="w-4 h-4 text-white" />
			</div>
		{/if}
	</div>

	<!-- Navigation -->
	<nav class="flex-1 py-2 px-2 space-y-0.5 overflow-y-auto">
		{#each navItems as item}
			{@const Icon = item.icon}
			{@const active = app.activeTab === item.id}
			<button
				onclick={() => navigate(item.id)}
				class="w-full flex items-center gap-3 rounded-lg transition text-sm
					{collapsed ? 'justify-center px-2 py-2.5' : 'px-3 py-2.5'}
					{active
					? 'bg-primary/10 text-primary font-semibold'
					: 'text-content-secondary hover:bg-surface-raised hover:text-content'}"
				title={collapsed ? item.label : undefined}
			>
				<Icon class="w-[18px] h-[18px] shrink-0" />
				{#if !collapsed}
					<span class="flex-1 text-left truncate">{item.label}</span>
					<span class="text-[10px] badge badge-neutral">{item.count()}</span>
				{/if}
			</button>
		{/each}
	</nav>

	<!-- Footer actions -->
	<div class="border-t border-edge px-2 py-2 space-y-0.5 shrink-0">
		<button
			onclick={() => app.syncCameras()}
			class="w-full flex items-center gap-3 rounded-lg text-sm text-content-secondary hover:bg-surface-raised hover:text-content transition
				{collapsed ? 'justify-center px-2 py-2' : 'px-3 py-2'}"
			title="Sincronizar cámaras"
		>
			<RefreshCw class="w-[18px] h-[18px] shrink-0" />
			{#if !collapsed}<span class="truncate">Sincronizar</span>{/if}
		</button>
		<button
			onclick={() => theme.toggle()}
			class="w-full flex items-center gap-3 rounded-lg text-sm text-content-secondary hover:bg-surface-raised hover:text-content transition
				{collapsed ? 'justify-center px-2 py-2' : 'px-3 py-2'}"
			title={theme.isDark ? 'Modo claro' : 'Modo oscuro'}
		>
			{#if theme.isDark}<Sun class="w-[18px] h-[18px] shrink-0" />{:else}<Moon class="w-[18px] h-[18px] shrink-0" />{/if}
			{#if !collapsed}<span class="truncate">{theme.isDark ? 'Modo claro' : 'Modo oscuro'}</span>{/if}
		</button>
		<button
			onclick={() => auth.logout()}
			class="w-full flex items-center gap-3 rounded-lg text-sm text-destructive hover:bg-surface-raised transition
				{collapsed ? 'justify-center px-2 py-2' : 'px-3 py-2'}"
			title="Cerrar sesión"
		>
			<LogOut class="w-[18px] h-[18px] shrink-0" />
			{#if !collapsed}<span class="truncate">Cerrar sesión</span>{/if}
		</button>
	</div>

	<!-- Collapse toggle (desktop only) -->
	<div class="hidden lg:flex border-t border-edge px-2 py-2 shrink-0">
		<button
			onclick={() => (collapsed = !collapsed)}
			class="w-full flex items-center gap-3 rounded-lg text-xs text-content-muted hover:bg-surface-raised hover:text-content transition
				{collapsed ? 'justify-center px-2 py-2' : 'px-3 py-2'}"
			title={collapsed ? 'Expandir sidebar' : 'Colapsar sidebar'}
		>
			{#if collapsed}
				<PanelLeft class="w-4 h-4 shrink-0" />
			{:else}
				<PanelLeftClose class="w-4 h-4 shrink-0" />
				<span class="truncate">Colapsar</span>
			{/if}
		</button>
	</div>
</aside>
