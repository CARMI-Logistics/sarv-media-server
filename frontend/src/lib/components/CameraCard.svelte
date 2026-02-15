<script lang="ts">
	import type { Camera } from '$lib/types';
	import { Eye, Trash2, Globe, User, Link, Play, Pencil, MapPin, Layers } from 'lucide-svelte';

	let {
		camera,
		onEdit,
		onDelete,
		onView
	}: {
		camera: Camera;
		onEdit: (id: number) => void;
		onDelete: (id: number) => void;
		onView: (id: number) => void;
	} = $props();
</script>

<div class="bg-gray-900 border border-gray-800 rounded-xl p-4 transition hover:-translate-y-0.5 hover:shadow-lg hover:shadow-black/15">
	<div class="flex items-start justify-between mb-3">
		<div class="flex items-center gap-2 min-w-0">
			<span class="status-dot {camera.enabled ? 'on' : 'off'} shrink-0"></span>
			<h4 class="font-medium text-sm text-white truncate">{camera.name}</h4>
		</div>
		<div class="flex gap-0.5 shrink-0">
			<button onclick={() => onEdit(camera.id)} class="p-1.5 text-gray-500 hover:text-amber-400 transition rounded hover:bg-gray-800" title="Editar">
				<Pencil class="w-3.5 h-3.5" />
			</button>
			<button onclick={() => onView(camera.id)} class="p-1.5 text-gray-500 hover:text-blue-400 transition rounded hover:bg-gray-800" title="Ver en vivo">
				<Eye class="w-3.5 h-3.5" />
			</button>
			<button onclick={() => onDelete(camera.id)} class="p-1.5 text-gray-500 hover:text-red-400 transition rounded hover:bg-gray-800" title="Eliminar">
				<Trash2 class="w-3.5 h-3.5" />
			</button>
		</div>
	</div>
	<div class="space-y-1.5 text-xs text-gray-400">
		<div class="flex items-center gap-2"><Globe class="w-3 h-3 shrink-0" /> {camera.host}:{camera.port}</div>
		<div class="flex items-center gap-2"><User class="w-3 h-3 shrink-0" /> {camera.username || '(sin auth)'}</div>
		<div class="flex items-center gap-2"><Link class="w-3 h-3 shrink-0" /><span class="truncate">{camera.path}</span></div>
	</div>
	{#if camera.location || camera.area}
		<div class="mt-2 flex items-center gap-2 flex-wrap text-xs">
			{#if camera.location}
				<span class="inline-flex items-center gap-1 px-2 py-0.5 bg-indigo-900/30 text-indigo-400 rounded-full">
					<MapPin class="w-2.5 h-2.5" /> {camera.location}
				</span>
			{/if}
			{#if camera.area}
				<span class="inline-flex items-center gap-1 px-2 py-0.5 bg-teal-900/30 text-teal-400 rounded-full">
					<Layers class="w-2.5 h-2.5" /> {camera.area}
				</span>
			{/if}
		</div>
	{/if}
	<div class="mt-2 flex items-center gap-2 flex-wrap">
		{#if camera.record}
			<span class="text-[10px] px-2 py-0.5 bg-green-900/40 text-green-400 rounded-full">REC</span>
		{/if}
		{#if camera.source_on_demand}
			<span class="text-[10px] px-2 py-0.5 bg-blue-900/40 text-blue-400 rounded-full">ON-DEMAND</span>
		{:else}
			<span class="text-[10px] px-2 py-0.5 bg-yellow-900/40 text-yellow-400 rounded-full">24/7</span>
		{/if}
		<span class="text-[10px] px-2 py-0.5 bg-gray-800 text-gray-400 rounded-full uppercase">{camera.protocol}</span>
		<button
			onclick={() => onView(camera.id)}
			class="ml-auto text-[10px] px-2 py-0.5 bg-blue-600/30 text-blue-400 rounded-full hover:bg-blue-600/50 transition cursor-pointer flex items-center gap-1"
		>
			<Play class="w-2.5 h-2.5" /> Ver
		</button>
	</div>
</div>
