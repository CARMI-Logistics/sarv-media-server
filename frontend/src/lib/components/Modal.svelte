<script lang="ts">
	import { X } from 'lucide-svelte';
	import type { Snippet } from 'svelte';

	let {
		open = $bindable(false),
		title = '',
		maxWidth = 'max-w-lg',
		children
	}: {
		open: boolean;
		title: string;
		maxWidth?: string;
		children: Snippet;
	} = $props();

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') open = false;
	}

	function handleBackdrop(e: MouseEvent) {
		if (e.target === e.currentTarget) open = false;
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center modal-overlay bg-black/50"
		onclick={handleBackdrop}
	>
		<div class="bg-gray-900 border border-gray-700 rounded-xl w-full {maxWidth} mx-4 fade-in shadow-2xl max-h-[90vh] overflow-y-auto">
			<div class="flex items-center justify-between px-6 py-4 border-b border-gray-800 sticky top-0 bg-gray-900 z-10">
				<h3 class="text-lg font-semibold">{title}</h3>
				<button onclick={() => (open = false)} class="text-gray-400 hover:text-white transition">
					<X class="w-5 h-5" />
				</button>
			</div>
			<div class="p-6">
				{@render children()}
			</div>
		</div>
	</div>
{/if}
