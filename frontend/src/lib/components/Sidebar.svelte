<script lang="ts">
	import { app } from '$lib/stores/app.svelte';
	import { auth } from '$lib/stores/auth.svelte';
	import { theme } from '$lib/stores/theme.svelte';
	import { brand } from '$lib/brand.config';
	import type { Tab } from '$lib/types';
	import {
		Video, Grid3x3, MapPin, Users, Sun, Moon, LogOut, RefreshCw,
		ChevronLeft, ChevronRight, PanelLeftClose, PanelLeft, X,
		Camera, Bell, Shield, User
	} from 'lucide-svelte';

	let {
		collapsed = $bindable(false),
		mobileOpen = $bindable(false),
	}: {
		collapsed: boolean;
		mobileOpen: boolean;
	} = $props();

	const navItems: { id: Tab; label: string; icon: typeof Video; count?: () => number | string; badge?: () => number }[] = [
		{ id: 'cameras', label: 'Cámaras', icon: Video, count: () => app.cameras.length },
		{ id: 'mosaics', label: 'Mosaicos', icon: Grid3x3, count: () => app.mosaics.length },
		{ id: 'locations', label: 'Ubicaciones', icon: MapPin, count: () => app.locations.length },
		{ id: 'captures', label: 'Capturas', icon: Camera, count: () => app.captures.length },
		{ id: 'notifications', label: 'Notificaciones', icon: Bell, badge: () => app.unreadCount },
		{ id: 'users', label: 'Usuarios', icon: Users, count: () => app.users.length },
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
		{collapsed ? 'w-[72px]' : 'w-64'}"
	style="box-shadow: 2px 0 8px var(--th-shadow);"
>
	<!-- Header -->
	<div class="flex items-center gap-3 px-5 py-5 border-b border-edge shrink-0 {collapsed ? 'justify-center px-3' : ''}">
		{#if !collapsed}
			<div class="w-9 h-9 bg-gradient-to-br from-blue-500 to-blue-600 rounded-xl flex items-center justify-center shrink-0 shadow-md">
				<Video class="w-5 h-5 text-white" />
			</div>
			<div class="min-w-0 flex-1">
				<h1 class="text-sm font-bold text-content truncate">{brand.name}</h1>
				<p class="text-[11px] text-content-muted truncate">{brand.tagline}</p>
			</div>
			<button onclick={() => (mobileOpen = false)} class="lg:hidden p-1.5 text-content-muted hover:text-content rounded-md hover:bg-surface-raised transition">
				<X class="w-4 h-4" />
			</button>
		{:else}
			<div class="w-9 h-9 bg-gradient-to-br from-blue-500 to-blue-600 rounded-xl flex items-center justify-center shadow-md">
				<Video class="w-5 h-5 text-white" />
			</div>
		{/if}
	</div>

	<!-- Navigation -->
	<nav class="flex-1 py-3 px-3 space-y-1 overflow-y-auto">
		{#each navItems as item}
			{@const Icon = item.icon}
			{@const active = app.activeTab === item.id}
			{@const badgeCount = item.badge ? item.badge() : 0}
			<button
				onclick={() => navigate(item.id)}
				class="w-full flex items-center gap-3.5 rounded-lg transition-all text-[13px] font-medium relative group
					{collapsed ? 'justify-center px-2 py-3' : 'px-3.5 py-3'}
					{active
					? 'bg-primary/[0.08] text-primary shadow-sm'
					: 'text-content-secondary hover:bg-surface-raised hover:text-content'}"
				title={collapsed ? item.label : undefined}
			>
				<Icon class="w-5 h-5 shrink-0 {active ? 'stroke-[2.5]' : ''}" />
				{#if !collapsed}
					<span class="flex-1 text-left truncate">{item.label}</span>
					{#if item.count}
						<span class="text-[10px] px-1.5 py-0.5 rounded bg-surface-raised text-content-muted font-normal">{item.count()}</span>
					{/if}
					{#if badgeCount > 0}
						<span class="absolute -top-1 -right-1 w-5 h-5 bg-red-500 text-white text-[10px] font-semibold rounded-full flex items-center justify-center shadow-md">
							{badgeCount > 99 ? '99+' : badgeCount}
						</span>
					{/if}
				{:else if badgeCount > 0}
					<span class="absolute -top-1 -right-1 w-2 h-2 bg-red-500 rounded-full shadow-md"></span>
				{/if}
			</button>
		{/each}
	</nav>

	<!-- Footer actions -->
	<div class="border-t border-edge px-3 py-2 space-y-1 shrink-0">
		<button
			onclick={() => app.syncCameras()}
			class="w-full flex items-center gap-3.5 rounded-lg text-[13px] font-medium text-content-secondary hover:bg-surface-raised hover:text-content transition-all
				{collapsed ? 'justify-center px-2 py-2.5' : 'px-3.5 py-2.5'}"
			title="Sincronizar cámaras"
		>
			<RefreshCw class="w-[18px] h-[18px] shrink-0" />
			{#if !collapsed}<span class="truncate">Sincronizar</span>{/if}
		</button>
		<button
			onclick={() => theme.toggle()}
			class="w-full flex items-center gap-3.5 rounded-lg text-[13px] font-medium text-content-secondary hover:bg-surface-raised hover:text-content transition-all
				{collapsed ? 'justify-center px-2 py-2.5' : 'px-3.5 py-2.5'}"
			title={theme.isDark ? 'Modo claro' : 'Modo oscuro'}
		>
			{#if theme.isDark}<Sun class="w-[18px] h-[18px] shrink-0" />{:else}<Moon class="w-[18px] h-[18px] shrink-0" />{/if}
			{#if !collapsed}<span class="truncate">{theme.isDark ? 'Modo claro' : 'Modo oscuro'}</span>{/if}
		</button>
	</div>

	<!-- User info & Logout -->
	<div class="border-t border-edge px-3 py-3 shrink-0">
		{#if !collapsed}
			<div class="flex items-center gap-3 px-3.5 py-2.5 rounded-lg bg-surface-raised mb-2">
				<div class="w-8 h-8 rounded-full bg-gradient-to-br from-blue-400 to-blue-600 flex items-center justify-center shrink-0 shadow-sm">
					<User class="w-4 h-4 text-white" />
				</div>
				<div class="min-w-0 flex-1">
					<p class="text-[13px] font-medium text-content truncate">Administrador</p>
					<p class="text-[11px] text-content-muted truncate">admin</p>
				</div>
			</div>
		{:else}
			<div class="flex justify-center mb-2">
				<div class="w-8 h-8 rounded-full bg-gradient-to-br from-blue-400 to-blue-600 flex items-center justify-center shadow-sm">
					<User class="w-4 h-4 text-white" />
				</div>
			</div>
		{/if}
		<button
			onclick={() => auth.logout()}
			class="w-full flex items-center gap-3.5 rounded-lg text-[13px] font-medium text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/30 transition-all
				{collapsed ? 'justify-center px-2 py-2.5' : 'px-3.5 py-2.5'}"
			title="Cerrar sesión"
		>
			<LogOut class="w-[18px] h-[18px] shrink-0" />
			{#if !collapsed}<span class="truncate">Cerrar sesión</span>{/if}
		</button>
	</div>

	<!-- Collapse toggle (desktop only) -->
	<div class="hidden lg:flex border-t border-edge px-3 py-2 shrink-0">
		<button
			onclick={() => (collapsed = !collapsed)}
			class="w-full flex items-center gap-3 rounded-lg text-xs text-content-muted hover:bg-surface-raised hover:text-content transition-all
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
