<script lang="ts">
	import { toast } from '$lib/stores/toast.svelte';
	import { X } from 'lucide-svelte';
</script>

{#if toast.items.length > 0}
	<div class="fixed bottom-6 right-6 z-50 flex flex-col gap-2">
		{#each toast.items as item (item.id)}
			<div
				class="fade-in flex items-center gap-3 px-4 py-3 rounded-lg text-sm shadow-lg min-w-[280px] {item.type === 'success'
					? 'bg-green-800 text-green-100'
					: item.type === 'error'
						? 'bg-red-800 text-red-100'
						: 'bg-blue-800 text-blue-100'}"
			>
				<span class="flex-1">{item.message}</span>
				<button onclick={() => toast.dismiss(item.id)} class="opacity-70 hover:opacity-100 transition">
					<X class="w-4 h-4" />
				</button>
			</div>
		{/each}
	</div>
{/if}
