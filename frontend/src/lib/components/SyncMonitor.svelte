<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { apiGet } from '$lib/api';
	import type { SyncStatus, SyncLog } from '$lib/types';
	import { Cloud, CloudOff, AlertCircle, CheckCircle, Clock, Database, FileText } from 'lucide-svelte';

	let status: SyncStatus | null = null;
	let logs: SyncLog[] = [];
	let showLogs = false;
	let loading = true;
	let interval: ReturnType<typeof setInterval> | null = null;

	async function loadStatus() {
		try {
			const json = await apiGet<SyncStatus>('/api/sync/status');
			if (json.success && json.data) {
				status = json.data;
			}
		} catch (error) {
			console.error('Error loading sync status:', error);
		} finally {
			loading = false;
		}
	}

	async function loadLogs() {
		try {
			const json = await apiGet<SyncLog[]>('/api/sync/logs');
			if (json.success && json.data) {
				logs = json.data;
			}
		} catch (error) {
			console.error('Error loading sync logs:', error);
		}
	}

	function formatTime(seconds: number | null): string {
		if (!seconds) return '-';
		const minutes = Math.floor(seconds / 60);
		const secs = seconds % 60;
		if (minutes > 60) {
			const hours = Math.floor(minutes / 60);
			const mins = minutes % 60;
			return `${hours}h ${mins}m`;
		}
		return `${minutes}m ${secs}s`;
	}

	function getLevelColor(level: string): string {
		switch (level.toUpperCase()) {
			case 'ERROR':
				return 'text-destructive';
			case 'WARN':
			case 'WARNING':
				return 'text-amber-500';
			case 'NOTICE':
				return 'text-primary';
			default:
				return 'text-content-secondary';
		}
	}

	function getLevelIcon(level: string) {
		switch (level.toUpperCase()) {
			case 'ERROR':
				return AlertCircle;
			case 'WARN':
			case 'WARNING':
				return AlertCircle;
			default:
				return CheckCircle;
		}
	}

	onMount(() => {
		loadStatus();
		loadLogs();
		
		// Refresh every 10 seconds
		interval = setInterval(() => {
			loadStatus();
			if (showLogs) {
				loadLogs();
			}
		}, 10000);
	});

	onDestroy(() => {
		if (interval) {
			clearInterval(interval);
		}
	});

	$: statusColor = status?.is_running 
		? 'text-success' 
		: status?.errors && status.errors > 10 
			? 'text-destructive' 
			: 'text-content-muted';

	$: StatusIcon = status?.is_running ? Cloud : CloudOff;
</script>

<div class="bg-surface-alt border border-edge rounded-xl p-4 space-y-4">
	<!-- Header -->
	<div class="flex items-center justify-between">
		<div class="flex items-center gap-3">
			<div class="p-2 bg-surface-raised rounded-lg">
				<svelte:component this={StatusIcon} class="w-5 h-5 {statusColor}" />
			</div>
			<div>
				<h3 class="font-semibold text-sm text-content">Sincronización S3</h3>
				<p class="text-xs text-content-secondary">
					{#if loading}
						Cargando...
					{:else if status}
						{status.status_message}
					{:else}
						No disponible
					{/if}
				</p>
			</div>
		</div>
		
		<button 
			onclick={() => showLogs = !showLogs}
			class="badge badge-neutral cursor-pointer hover:opacity-80 transition inline-flex items-center gap-1.5"
		>
			<FileText class="w-3 h-3" />
			{showLogs ? 'Ocultar' : 'Ver'} Logs
		</button>
	</div>

	{#if !loading && status}
		<!-- Stats Grid -->
		<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
			<!-- Files Synced -->
			<div class="bg-surface-raised rounded-lg p-3">
				<div class="flex items-center gap-2 mb-1">
					<Database class="w-4 h-4 text-content-muted" />
					<span class="text-[10px] text-content-muted uppercase tracking-wide">Archivos</span>
				</div>
				<p class="text-lg font-semibold text-content">{status.files_synced.toLocaleString()}</p>
			</div>

			<!-- Total Size -->
			<div class="bg-surface-raised rounded-lg p-3">
				<div class="flex items-center gap-2 mb-1">
					<Database class="w-4 h-4 text-content-muted" />
					<span class="text-[10px] text-content-muted uppercase tracking-wide">Tamaño</span>
				</div>
				<p class="text-lg font-semibold text-content">{status.total_size_gb.toFixed(1)} GB</p>
			</div>

			<!-- Last Sync -->
			<div class="bg-surface-raised rounded-lg p-3">
				<div class="flex items-center gap-2 mb-1">
					<Clock class="w-4 h-4 text-content-muted" />
					<span class="text-[10px] text-content-muted uppercase tracking-wide">Última Sync</span>
				</div>
				<p class="text-sm font-medium text-content truncate">{status.last_sync || 'Nunca'}</p>
			</div>

			<!-- Next Sync -->
			<div class="bg-surface-raised rounded-lg p-3">
				<div class="flex items-center gap-2 mb-1">
					<Clock class="w-4 h-4 text-content-muted" />
					<span class="text-[10px] text-content-muted uppercase tracking-wide">Próxima Sync</span>
				</div>
				<p class="text-sm font-medium text-content truncate">
					{status.is_running ? formatTime(status.next_sync_in) : 'Detenido'}
				</p>
			</div>
		</div>

		<!-- Errors Warning -->
		{#if status.errors > 0}
			<div class="flex items-center gap-2 px-3 py-2 bg-amber-500/10 border border-amber-500/20 rounded-lg">
				<AlertCircle class="w-4 h-4 text-amber-500 shrink-0" />
				<p class="text-xs text-amber-500">
					{status.errors} error{status.errors !== 1 ? 'es' : ''} detectado{status.errors !== 1 ? 's' : ''} 
					(archivos en grabación activa se reintentarán en el próximo ciclo)
				</p>
			</div>
		{/if}

		<!-- Logs Panel -->
		{#if showLogs}
			<div class="border-t border-edge pt-4 space-y-2">
				<h4 class="text-xs font-semibold text-content-secondary uppercase tracking-wide mb-3">
					Bitácora de Sincronización (Últimos 100 eventos)
				</h4>
				
				<div class="bg-surface-raised rounded-lg border border-edge max-h-64 overflow-y-auto">
					{#if logs.length === 0}
						<p class="text-xs text-content-muted text-center py-8">No hay logs disponibles</p>
					{:else}
						<div class="divide-y divide-edge">
							{#each logs as log}
								<div class="px-3 py-2 hover:bg-surface-alt/50 transition">
									<div class="flex items-start gap-2">
										<svelte:component 
											this={getLevelIcon(log.level)} 
											class="w-3 h-3 mt-0.5 shrink-0 {getLevelColor(log.level)}" 
										/>
										<div class="flex-1 min-w-0">
											<div class="flex items-center gap-2 mb-0.5">
												<span class="text-[10px] {getLevelColor(log.level)} font-medium uppercase">
													{log.level}
												</span>
												{#if log.timestamp}
													<span class="text-[10px] text-content-muted">
														{log.timestamp}
													</span>
												{/if}
											</div>
											<p class="text-xs text-content-secondary leading-relaxed break-words">
												{log.message}
											</p>
										</div>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		{/if}
	{/if}
</div>

<style>
	/* Custom scrollbar for logs */
	.overflow-y-auto::-webkit-scrollbar {
		width: 6px;
	}
	
	.overflow-y-auto::-webkit-scrollbar-track {
		background: var(--th-surface-raised);
		border-radius: 3px;
	}
	
	.overflow-y-auto::-webkit-scrollbar-thumb {
		background: var(--th-edge);
		border-radius: 3px;
	}
	
	.overflow-y-auto::-webkit-scrollbar-thumb:hover {
		background: var(--th-content-muted);
	}
</style>
