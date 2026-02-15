import { apiGet, apiPost, apiPut, apiDelete } from '$lib/api';
import { toast } from '$lib/stores/toast.svelte';
import type { Camera, Location, AreaWithLocation, Mosaic, Tab } from '$lib/types';

class AppStore {
	cameras = $state<Camera[]>([]);
	locations = $state<Location[]>([]);
	areas = $state<AreaWithLocation[]>([]);
	mosaics = $state<Mosaic[]>([]);
	activeTab = $state<Tab>('cameras');
	searchQuery = $state('');

	// Filters
	filterLocations = $state<string[]>([]);
	filterAreas = $state<string[]>([]);
	filterEnabled = $state(true);
	filterRecording = $state(true);

	filteredCameras = $derived.by(() => {
		const search = this.searchQuery.toLowerCase();
		return this.cameras.filter((cam) => {
			const matchesSearch =
				!search ||
				cam.name.toLowerCase().includes(search) ||
				cam.host.toLowerCase().includes(search) ||
				(cam.location || '').toLowerCase().includes(search) ||
				(cam.area || '').toLowerCase().includes(search);
			const matchesLocation =
				this.filterLocations.length === 0 || this.filterLocations.includes(cam.location);
			const matchesArea =
				this.filterAreas.length === 0 || this.filterAreas.includes(cam.area);
			const matchesEnabled = !this.filterEnabled || cam.enabled;
			const matchesRecording = !this.filterRecording || cam.record;
			return matchesSearch && matchesLocation && matchesArea && matchesEnabled && matchesRecording;
		});
	});

	clearFilters() {
		this.filterLocations = [];
		this.filterAreas = [];
		this.filterEnabled = true;
		this.filterRecording = true;
		this.searchQuery = '';
	}

	// ── Cameras ──
	async loadCameras() {
		try {
			const json = await apiGet<Camera[]>('/api/cameras');
			if (json.success && json.data) this.cameras = json.data;
		} catch {
			toast.error('Error cargando cámaras');
		}
	}

	async saveCamera(id: number | null, body: Partial<Camera>) {
		try {
			const json = id
				? await apiPut<boolean>(`/api/cameras/${id}`, body)
				: await apiPost<number>('/api/cameras', body);
			if (json.success) {
				await this.loadCameras();
				toast.success(id ? 'Cámara actualizada' : 'Cámara creada');
				return true;
			}
			toast.error(json.error || 'Error guardando cámara');
			return false;
		} catch {
			toast.error('Error de conexión');
			return false;
		}
	}

	async deleteCamera(id: number) {
		try {
			const json = await apiDelete<boolean>(`/api/cameras/${id}`);
			if (json.success) {
				await this.loadCameras();
				toast.success('Cámara eliminada');
			} else {
				toast.error(json.error || 'Error eliminando');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	async syncCameras() {
		try {
			toast.info('Sincronizando...');
			const json = await apiPost<string>('/api/cameras/sync');
			if (json.success) toast.success(json.data || 'Sincronizado');
			else toast.error(json.error || 'Error');
		} catch {
			toast.error('Error de conexión');
		}
	}

	// ── Locations ──
	async loadLocations() {
		try {
			const json = await apiGet<Location[]>('/api/locations');
			if (json.success && json.data) this.locations = json.data;
		} catch {
			toast.error('Error cargando ubicaciones');
		}
	}

	async saveLocation(id: number | null, body: Partial<Location>) {
		try {
			const json = id
				? await apiPut<boolean>(`/api/locations/${id}`, body)
				: await apiPost<number>('/api/locations', body);
			if (json.success) {
				await this.loadLocations();
				toast.success(id ? 'Ubicación actualizada' : 'Ubicación creada');
				return true;
			}
			return json.error || 'Error guardando ubicación';
		} catch {
			return 'Error de conexión';
		}
	}

	async deleteLocation(id: number) {
		const loc = this.locations.find((l) => l.id === id);
		if (loc?.is_system) {
			toast.error('No se pueden eliminar ubicaciones del sistema');
			return;
		}
		try {
			const json = await apiDelete<boolean>(`/api/locations/${id}`);
			if (json.success) {
				await this.loadLocations();
				await this.loadAreas();
				toast.success('Ubicación eliminada');
			} else {
				toast.error(json.error || 'Error eliminando');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	// ── Areas ──
	async loadAreas() {
		try {
			const json = await apiGet<AreaWithLocation[]>('/api/areas');
			if (json.success && json.data) this.areas = json.data;
		} catch {
			toast.error('Error cargando áreas');
		}
	}

	async saveArea(id: number | null, body: { name: string; location_id: number; description: string }) {
		try {
			const json = id
				? await apiPut<boolean>(`/api/areas/${id}`, body)
				: await apiPost<number>('/api/areas', body);
			if (json.success) {
				await this.loadAreas();
				toast.success(id ? 'Área actualizada' : 'Área creada');
				return true;
			}
			return json.error || 'Error guardando área';
		} catch {
			return 'Error de conexión';
		}
	}

	async deleteArea(id: number) {
		try {
			const json = await apiDelete<boolean>(`/api/areas/${id}`);
			if (json.success) {
				await this.loadAreas();
				toast.success('Área eliminada');
			} else {
				toast.error(json.error || 'Error eliminando');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	// ── Mosaics ──
	async loadMosaics() {
		try {
			const json = await apiGet<Mosaic[]>('/api/mosaics');
			if (json.success && json.data) this.mosaics = json.data;
		} catch { /* ignore */ }
	}

	async saveMosaic(id: number | null, body: { name: string; layout: string; camera_ids: number[] }) {
		try {
			const json = id
				? await apiPut<boolean>(`/api/mosaics/${id}`, body)
				: await apiPost<number>('/api/mosaics', body);
			if (json.success) {
				await this.loadMosaics();
				toast.success(id ? 'Mosaico actualizado' : 'Mosaico creado');
				return true;
			}
			return json.error || 'Error guardando mosaico';
		} catch {
			return 'Error de conexión';
		}
	}

	async deleteMosaic(id: number) {
		try {
			const json = await apiDelete<boolean>(`/api/mosaics/${id}`);
			if (json.success) {
				await this.loadMosaics();
				toast.success('Mosaico eliminado');
			} else {
				toast.error(json.error || 'Error');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	async startMosaic(id: number) {
		try {
			toast.info('Iniciando mosaico...');
			const json = await apiPost<string>(`/api/mosaics/${id}/start`);
			if (json.success) {
				toast.success(json.data || 'Mosaico iniciado');
				await this.loadMosaics();
			} else {
				toast.error(json.error || 'Error');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	async stopMosaic(id: number) {
		try {
			const json = await apiPost<string>(`/api/mosaics/${id}/stop`);
			if (json.success) {
				toast.success('Mosaico detenido');
				await this.loadMosaics();
			} else {
				toast.error(json.error || 'Error');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	// ── Init ──
	async loadAll() {
		await Promise.all([
			this.loadLocations(),
			this.loadAreas(),
			this.loadCameras(),
			this.loadMosaics()
		]);
	}
}

export const app = new AppStore();
