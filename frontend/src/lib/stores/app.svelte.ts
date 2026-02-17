import { apiGet, apiPost, apiPut, apiDelete } from '$lib/api';
import { toast } from '$lib/stores/toast.svelte';
import type { Camera, Location, AreaWithLocation, Mosaic, Tab, UserPublic, Capture, NotificationSummary, Notification, RoleWithPermissions, MosaicShare } from '$lib/types';

class AppStore {
	cameras = $state<Camera[]>([]);
	locations = $state<Location[]>([]);
	areas = $state<AreaWithLocation[]>([]);
	mosaics = $state<Mosaic[]>([]);
	users = $state<UserPublic[]>([]);
	captures = $state<Capture[]>([]);
	notifications = $state<Notification[]>([]);
	unreadCount = $state(0);
	roles = $state<RoleWithPermissions[]>([]);
	shares = $state<MosaicShare[]>([]);
	thumbnailsEnabled = $state(true);
	activeTab = $state<Tab>('cameras');
	searchQuery = $state('');
	cameraStatuses = $state<Record<string, 'online' | 'offline' | 'unknown' | 'disabled'>>({});
	private statusInterval: ReturnType<typeof setInterval> | null = null;
	private notifInterval: ReturnType<typeof setInterval> | null = null;

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

	async loadCameraThumbnail(cameraId: number) {
		try {
			const json = await apiGet<{ camera_id: number; thumbnail_url: string | null }>(`/api/cameras/${cameraId}/thumbnail`);
			if (json.success && json.data) {
				const camera = this.cameras.find(c => c.id === cameraId);
				if (camera) {
					camera.thumbnail_url = json.data.thumbnail_url;
				}
			}
		} catch {
			// Silently fail for thumbnails
		}
	}

	async loadAllThumbnails() {
		for (const camera of this.cameras) {
			await this.loadCameraThumbnail(camera.id);
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

	// ── Users ──
	async loadUsers() {
		try {
			const json = await apiGet<UserPublic[]>('/api/users');
			if (json.success) this.users = json.data;
		} catch { /* ignore */ }
	}

	async saveUser(id: number | null, body: Record<string, unknown>): Promise<boolean> {
		try {
			const json = id
				? await apiPut<UserPublic>(`/api/users/${id}`, body)
				: await apiPost<UserPublic>('/api/users', body);
			if (json.success) {
				toast.success(id ? 'Usuario actualizado' : 'Usuario creado');
				await this.loadUsers();
				return true;
			} else {
				toast.error(json.error || 'Error');
				return false;
			}
		} catch {
			toast.error('Error de conexión');
			return false;
		}
	}

	async deleteUser(id: number) {
		try {
			const json = await apiDelete<string>(`/api/users/${id}`);
			if (json.success) {
				toast.success('Usuario eliminado');
				await this.loadUsers();
			} else {
				toast.error(json.error || 'Error');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	// ── Camera Status ──
	async checkCameraStatuses() {
		try {
			const json = await apiGet<{ name: string; ready: boolean }[]>('/api/cameras/status');
			if (json.success && json.data) {
				const statuses: Record<string, 'online' | 'offline' | 'unknown' | 'disabled'> = {};
				for (const cam of this.cameras) {
					if (!cam.enabled) {
						statuses[cam.name] = 'disabled';
					} else {
						const found = json.data.find((p) => p.name === cam.name);
						statuses[cam.name] = found ? (found.ready ? 'online' : 'offline') : 'offline';
					}
				}
				this.cameraStatuses = statuses;
			}
		} catch {
			// Endpoint may not exist yet; mark all as unknown
			const statuses: Record<string, 'online' | 'offline' | 'unknown' | 'disabled'> = {};
			for (const cam of this.cameras) {
				statuses[cam.name] = cam.enabled ? 'unknown' : 'disabled';
			}
			this.cameraStatuses = statuses;
		}
	}

	startStatusPolling() {
		if (this.statusInterval) return;
		this.checkCameraStatuses();
		this.statusInterval = setInterval(() => this.checkCameraStatuses(), 30000);
	}

	stopStatusPolling() {
		if (this.statusInterval) {
			clearInterval(this.statusInterval);
			this.statusInterval = null;
		}
	}

	// ── Captures ──
	async loadCaptures(cameraId?: number, date?: string) {
		try {
			let url = '/api/captures?';
			if (cameraId) url += `camera_id=${cameraId}&`;
			if (date) url += `date=${date}&`;
			const json = await apiGet<Capture[]>(url);
			if (json.success && json.data) this.captures = json.data;
		} catch { /* ignore */ }
	}

	async takeScreenshot(cameraId: number) {
		try {
			toast.info('Capturando screenshot...');
			const json = await apiPost<Capture>(`/api/captures/screenshot/${cameraId}`);
			if (json.success) {
				toast.success('Screenshot capturado');
				await this.loadCaptures();
				return true;
			}
			toast.error(json.error || 'Error capturando');
			return false;
		} catch {
			toast.error('Error de conexión');
			return false;
		}
	}

	async deleteCapture(id: number) {
		try {
			const json = await apiDelete<string>(`/api/captures/${id}`);
			if (json.success) {
				toast.success('Captura eliminada');
				await this.loadCaptures();
			} else {
				toast.error(json.error || 'Error');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	async loadThumbnailSetting() {
		try {
			const json = await apiGet<boolean>('/api/captures/thumbnails/setting');
			if (json.success) this.thumbnailsEnabled = json.data;
		} catch { /* ignore */ }
	}

	async toggleThumbnails() {
		try {
			const json = await apiPost<boolean>('/api/captures/thumbnails/toggle');
			if (json.success) {
				this.thumbnailsEnabled = json.data;
				toast.success(json.data ? 'Miniaturas activadas' : 'Miniaturas desactivadas');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	// ── Notifications ──
	async loadNotifications() {
		try {
			const json = await apiGet<NotificationSummary>('/api/notifications/summary');
			if (json.success && json.data) {
				this.notifications = json.data.notifications;
				this.unreadCount = json.data.unread_count;
			}
		} catch { /* ignore */ }
	}

	async markNotificationRead(id: number) {
		try {
			await apiPost<string>(`/api/notifications/${id}/read`);
			await this.loadNotifications();
		} catch { /* ignore */ }
	}

	async markAllNotificationsRead() {
		try {
			await apiPost<string>('/api/notifications/read-all');
			this.unreadCount = 0;
			this.notifications = this.notifications.map((n) => ({ ...n, read: true }));
		} catch { /* ignore */ }
	}

	async deleteNotification(id: number) {
		try {
			await apiDelete<string>(`/api/notifications/${id}`);
			await this.loadNotifications();
		} catch { /* ignore */ }
	}

	startNotificationPolling() {
		if (this.notifInterval) return;
		this.loadNotifications();
		this.notifInterval = setInterval(() => this.loadNotifications(), 30000);
	}

	stopNotificationPolling() {
		if (this.notifInterval) {
			clearInterval(this.notifInterval);
			this.notifInterval = null;
		}
	}

	// ── Roles ──
	async loadRoles() {
		try {
			const json = await apiGet<RoleWithPermissions[]>('/api/roles');
			if (json.success && json.data) this.roles = json.data;
		} catch { /* ignore */ }
	}

	async saveRole(id: number | null, body: Record<string, unknown>): Promise<boolean> {
		try {
			const json = id
				? await apiPut<string>(`/api/roles/${id}`, body)
				: await apiPost<string>('/api/roles', body);
			if (json.success) {
				toast.success(id ? 'Rol actualizado' : 'Rol creado');
				await this.loadRoles();
				return true;
			} else {
				toast.error(json.error || 'Error');
				return false;
			}
		} catch {
			toast.error('Error de conexión');
			return false;
		}
	}

	async deleteRole(id: number) {
		try {
			const json = await apiDelete<string>(`/api/roles/${id}`);
			if (json.success) {
				toast.success('Rol eliminado');
				await this.loadRoles();
			} else {
				toast.error(json.error || 'Error');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	// ── Shares ──
	async loadShares(mosaicId?: number) {
		try {
			const url = mosaicId ? `/api/shares?mosaic_id=${mosaicId}` : '/api/shares';
			const json = await apiGet(url);
			if (json.success) this.shares = json.data as MosaicShare[];
		} catch { /* silent */ }
	}

	async createShare(data: { mosaic_id: number; emails: string[]; duration_hours: number; schedule_start?: string; schedule_end?: string }): Promise<true | string> {
		try {
			const json = await apiPost('/api/shares', data);
			if (json.success) {
				toast.success('Enlace compartido creado');
				await this.loadShares();
				return true;
			}
			return json.error || 'Error';
		} catch {
			return 'Error de conexión';
		}
	}

	async deleteShare(id: number) {
		try {
			const json = await apiDelete(`/api/shares/${id}`);
			if (json.success) {
				this.shares = this.shares.filter((s) => s.id !== id);
				toast.success('Enlace eliminado');
			} else {
				toast.error(json.error || 'Error');
			}
		} catch {
			toast.error('Error de conexión');
		}
	}

	async toggleShare(id: number) {
		try {
			const json = await apiPost(`/api/shares/${id}/toggle`, {});
			if (json.success) {
				await this.loadShares();
				toast.success('Estado actualizado');
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
			this.loadMosaics(),
			this.loadUsers(),
			this.loadRoles(),
			this.loadThumbnailSetting()
		]);
		this.startStatusPolling();
		this.startNotificationPolling();
	}

	stopAll() {
		this.stopStatusPolling();
		this.stopNotificationPolling();
	}
}

export const app = new AppStore();
