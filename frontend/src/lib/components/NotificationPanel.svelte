<script lang="ts">
	import { app } from '$lib/stores/app.svelte';
	import { Bell, BellOff, Check, CheckCheck, Trash2, X, WifiOff, HardDrive, AlertTriangle, Info, AlertCircle } from 'lucide-svelte';

	let {
		open = $bindable(false)
	}: {
		open: boolean;
	} = $props();

	const severityIcon: Record<string, typeof Info> = {
		info: Info,
		warning: AlertTriangle,
		error: AlertCircle,
		success: Check,
	};

	const severityColor: Record<string, string> = {
		info: 'text-blue-500',
		warning: 'text-amber-500',
		error: 'text-red-500',
		success: 'text-emerald-500',
	};

	const categoryIcon: Record<string, typeof Info> = {
		disconnect: WifiOff,
		storage: HardDrive,
		system: Info,
		camera: AlertTriangle,
	};

	function formatTime(dt: string | null): string {
		if (!dt) return '';
		const d = new Date(dt + 'Z');
		const now = new Date();
		const diff = now.getTime() - d.getTime();
		if (diff < 60000) return 'Ahora';
		if (diff < 3600000) return `Hace ${Math.floor(diff / 60000)}m`;
		if (diff < 86400000) return `Hace ${Math.floor(diff / 3600000)}h`;
		return d.toLocaleDateString();
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-50 flex justify-end" onclick={() => (open = false)}>
		<div class="absolute inset-0 bg-black/30 backdrop-blur-sm"></div>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="relative w-full max-w-sm bg-surface-alt border-l border-edge h-full flex flex-col fade-in shadow-2xl"
			onclick={(e) => e.stopPropagation()}
		>
			<!-- Header -->
			<div class="flex items-center justify-between px-4 py-3 border-b border-edge shrink-0">
				<div class="flex items-center gap-2">
					<Bell class="w-4 h-4 text-primary" />
					<h3 class="text-sm font-semibold text-content">Notificaciones</h3>
					{#if app.unreadCount > 0}
						<span class="badge badge-danger text-[10px] px-1.5 py-0.5">{app.unreadCount}</span>
					{/if}
				</div>
				<div class="flex items-center gap-1">
					{#if app.unreadCount > 0}
						<button
							onclick={() => app.markAllNotificationsRead()}
							class="p-1.5 text-content-muted hover:text-primary transition rounded-lg hover:bg-surface-raised"
							title="Marcar todas como leídas"
						>
							<CheckCheck class="w-4 h-4" />
						</button>
					{/if}
					<button onclick={() => (open = false)} class="p-1.5 text-content-muted hover:text-content transition rounded-lg hover:bg-surface-raised">
						<X class="w-4 h-4" />
					</button>
				</div>
			</div>

			<!-- Notifications list -->
			<div class="flex-1 overflow-y-auto">
				{#if app.notifications.length === 0}
					<div class="flex flex-col items-center justify-center h-full text-content-muted gap-3 px-6">
						<BellOff class="w-10 h-10 opacity-30" />
						<p class="text-sm text-center">No hay notificaciones</p>
					</div>
				{:else}
					<div class="divide-y divide-edge">
						{#each app.notifications as notif}
							{@const SevIcon = severityIcon[notif.severity] || Info}
							{@const CatIcon = categoryIcon[notif.category] || Info}
							<div class="px-4 py-3 hover:bg-surface-raised/50 transition {notif.read ? 'opacity-60' : ''}">
								<div class="flex items-start gap-3">
									<div class="shrink-0 mt-0.5">
										<CatIcon class="w-4 h-4 {severityColor[notif.severity] || 'text-content-muted'}" />
									</div>
									<div class="flex-1 min-w-0">
										<div class="flex items-center justify-between gap-2">
											<h4 class="text-sm font-medium text-content truncate {notif.read ? '' : 'font-semibold'}">{notif.title}</h4>
											<span class="text-[10px] text-content-muted shrink-0">{formatTime(notif.created_at)}</span>
										</div>
										{#if notif.message}
											<p class="text-xs text-content-secondary mt-0.5 line-clamp-2">{notif.message}</p>
										{/if}
										<div class="flex items-center gap-2 mt-1.5">
											<span class="text-[10px] badge badge-neutral capitalize">{notif.category}</span>
											{#if !notif.read}
												<button
													onclick={() => app.markNotificationRead(notif.id)}
													class="text-[10px] text-primary hover:underline flex items-center gap-0.5"
												>
													<Check class="w-3 h-3" /> Leída
												</button>
											{/if}
											<button
												onclick={() => app.deleteNotification(notif.id)}
												class="text-[10px] text-content-muted hover:text-destructive flex items-center gap-0.5 ml-auto"
											>
												<Trash2 class="w-3 h-3" />
											</button>
										</div>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}
