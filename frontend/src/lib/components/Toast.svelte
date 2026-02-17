<script lang="ts">
	import { toast } from '$lib/stores/toast.svelte';
	import { X, CheckCircle2, AlertCircle, Info } from 'lucide-svelte';
</script>

{#if toast.items.length > 0}
	<div class="fixed bottom-4 right-4 sm:bottom-6 sm:right-6 z-[60] flex flex-col gap-2 max-w-[calc(100vw-2rem)]">
		{#each toast.items as item (item.id)}
			<div
				class="fade-in flex items-center gap-3 px-4 py-3 rounded-xl text-sm shadow-xl min-w-[280px] max-w-sm border backdrop-blur-sm
					{item.type === 'success'
					? 'bg-emerald-50 text-emerald-800 border-emerald-200 dark:bg-emerald-900/40 dark:text-emerald-200 dark:border-emerald-800/50'
					: item.type === 'error'
						? 'bg-red-50 text-red-800 border-red-200 dark:bg-red-900/40 dark:text-red-200 dark:border-red-800/50'
						: 'bg-blue-50 text-blue-800 border-blue-200 dark:bg-blue-900/40 dark:text-blue-200 dark:border-blue-800/50'}"
			>
				{#if item.type === 'success'}
					<CheckCircle2 class="w-4 h-4 shrink-0" />
				{:else if item.type === 'error'}
					<AlertCircle class="w-4 h-4 shrink-0" />
				{:else}
					<Info class="w-4 h-4 shrink-0" />
				{/if}
				<span class="flex-1">{item.message}</span>
				<button onclick={() => toast.dismiss(item.id)} class="opacity-60 hover:opacity-100 transition shrink-0">
					<X class="w-4 h-4" />
				</button>
			</div>
		{/each}
	</div>
{/if}
