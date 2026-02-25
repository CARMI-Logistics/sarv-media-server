<script lang="ts">
	import { toast } from '$lib/stores/toast.svelte';
	import { X, CheckCircle2, AlertCircle, Info } from 'lucide-svelte';
	import { fly, fade, scale } from 'svelte/transition';
	import { quintOut } from 'svelte/easing';
</script>

{#if toast.items.length > 0}
	<div class="fixed bottom-4 right-4 sm:bottom-6 sm:right-6 z-[60] flex flex-col gap-2.5 max-w-[calc(100vw-2rem)] pointer-events-none">
		{#each toast.items as item (item.id)}
			<div
				in:fly={{ x: 100, duration: 400, easing: quintOut }}
				out:fly={{ x: 100, duration: 300, easing: quintOut }}
				class="flex items-start gap-3 px-4 py-3.5 rounded-xl text-sm font-medium shadow-2xl min-w-[300px] max-w-md border-2 pointer-events-auto
					transform hover:scale-[1.02] active:scale-100 transition-transform duration-200
					{item.type === 'success'
					? 'bg-gradient-to-br from-emerald-50 to-emerald-100/80 text-emerald-900 border-emerald-300 dark:from-emerald-900/50 dark:to-emerald-950/30 dark:text-emerald-100 dark:border-emerald-700/60'
					: item.type === 'error'
						? 'bg-gradient-to-br from-red-50 to-red-100/80 text-red-900 border-red-300 dark:from-red-900/50 dark:to-red-950/30 dark:text-red-100 dark:border-red-700/60'
						: 'bg-gradient-to-br from-blue-50 to-blue-100/80 text-blue-900 border-blue-300 dark:from-blue-900/50 dark:to-blue-950/30 dark:text-blue-100 dark:border-blue-700/60'}"
			>
				<div class="shrink-0 mt-0.5">
					{#if item.type === 'success'}
						<div class="w-5 h-5 rounded-full bg-emerald-500/20 flex items-center justify-center animate-pulse">
							<CheckCircle2 class="w-4 h-4 text-emerald-600 dark:text-emerald-400" />
						</div>
					{:else if item.type === 'error'}
						<div class="w-5 h-5 rounded-full bg-red-500/20 flex items-center justify-center">
							<AlertCircle class="w-4 h-4 text-red-600 dark:text-red-400" />
						</div>
					{:else}
						<div class="w-5 h-5 rounded-full bg-blue-500/20 flex items-center justify-center">
							<Info class="w-4 h-4 text-blue-600 dark:text-blue-400" />
						</div>
					{/if}
				</div>
				<span class="flex-1 leading-relaxed">{item.message}</span>
				<button 
					onclick={() => toast.dismiss(item.id)} 
					class="shrink-0 p-1 rounded-md hover:bg-black/10 dark:hover:bg-white/10 transition-colors opacity-60 hover:opacity-100"
					aria-label="Cerrar notificación"
				>
					<X class="w-4 h-4" />
				</button>
			</div>
		{/each}
	</div>
{/if}
