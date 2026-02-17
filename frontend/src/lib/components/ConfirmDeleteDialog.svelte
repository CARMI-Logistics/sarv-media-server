<script lang="ts">
	import Modal from './Modal.svelte';
	import { AlertTriangle } from 'lucide-svelte';

	let {
		open = $bindable(false),
		title = 'Confirmar eliminación',
		message = '¿Estás seguro de que deseas eliminar este elemento?',
		onConfirm
	}: {
		open: boolean;
		title?: string;
		message?: string;
		onConfirm: () => void;
	} = $props();

	const words = ['eliminar', 'borrar', 'confirmar', 'aceptar', 'proceder', 'continuar', 'seguro', 'definitivo'];
	let confirmWord = $state('');
	let inputValue = $state('');

	$effect(() => {
		if (open) {
			confirmWord = words[Math.floor(Math.random() * words.length)];
			inputValue = '';
		}
	});

	let isMatch = $derived(inputValue.toLowerCase().trim() === confirmWord);

	function handleConfirm() {
		if (!isMatch) return;
		open = false;
		onConfirm();
	}
</script>

<Modal bind:open {title} maxWidth="max-w-md">
	<div class="space-y-4">
		<div class="flex items-start gap-3">
			<div class="w-10 h-10 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center shrink-0">
				<AlertTriangle class="w-5 h-5 text-red-600 dark:text-red-400" />
			</div>
			<div>
				<p class="text-sm text-content-secondary">{message}</p>
				<p class="text-sm text-content-secondary mt-2">
					Escribe <strong class="text-destructive font-mono">{confirmWord}</strong> para confirmar:
				</p>
			</div>
		</div>
		<input
			type="text"
			bind:value={inputValue}
			class="input py-2.5 {isMatch ? 'input-success' : inputValue.length > 0 ? 'input-error' : ''}"
			placeholder="Escribe la palabra de confirmación"
			autofocus
		/>
		<div class="flex justify-end gap-3">
			<button type="button" onclick={() => (open = false)} class="btn btn-secondary">Cancelar</button>
			<button
				type="button"
				onclick={handleConfirm}
				disabled={!isMatch}
				class="btn bg-red-600 hover:bg-red-700 text-white disabled:opacity-40 disabled:cursor-not-allowed"
			>
				Eliminar
			</button>
		</div>
	</div>
</Modal>
