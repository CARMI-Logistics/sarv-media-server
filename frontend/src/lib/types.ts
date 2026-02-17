export interface Camera {
	id: number;
	name: string;
	host: string;
	port: number;
	username: string;
	password: string;
	path: string;
	protocol: string;
	enabled: boolean;
	record: boolean;
	source_on_demand: boolean;
	location: string;
	area: string;
	created_at: string;
	updated_at: string;
	thumbnail_url?: string | null;
}

export interface ThumbnailResponse {
	camera_id: number;
	thumbnail_url: string | null;
}

export interface SyncStatus {
	is_running: boolean;
	last_sync: string | null;
	next_sync_in: number | null;
	files_synced: number;
	total_size_gb: number;
	errors: number;
	status_message: string;
}

export interface SyncLog {
	timestamp: string;
	message: string;
	level: string;
}

export interface Location {
	id: number;
	name: string;
	description: string;
	is_system: boolean;
	created_at: string | null;
}

export interface Area {
	id: number;
	name: string;
	location_id: number;
	description: string;
	created_at: string | null;
}

export interface AreaWithLocation extends Area {
	location_name: string;
}

export interface Mosaic {
	id: number;
	name: string;
	layout: string;
	active: boolean;
	pid: number | null;
	cameras: MosaicCamera[];
	created_at: string;
}

export interface MosaicCamera {
	camera_id: number;
	camera_name: string;
	position: number;
}

export interface UserPublic {
	id: number;
	username: string;
	email: string;
	role: string;
	active: boolean;
	created_at: string | null;
	updated_at: string | null;
}

export interface Capture {
	id: number;
	camera_id: number;
	camera_name: string;
	capture_type: string;
	file_path: string;
	file_size: number;
	created_at: string | null;
}

export interface Notification {
	id: number;
	category: string;
	title: string;
	message: string;
	severity: string;
	read: boolean;
	created_at: string | null;
}

export interface NotificationSummary {
	unread_count: number;
	notifications: Notification[];
}

export interface Permission {
	id: number;
	role_id: number;
	module: string;
	can_view: boolean;
	can_create: boolean;
	can_edit: boolean;
	can_delete: boolean;
}

export interface Role {
	id: number;
	name: string;
	description: string;
	is_system: boolean;
	created_at: string | null;
}

export interface RoleWithPermissions extends Role {
	permissions: Permission[];
}

export interface PermissionInput {
	module: string;
	can_view: boolean;
	can_create: boolean;
	can_edit: boolean;
	can_delete: boolean;
}

export interface MosaicShare {
	id: number;
	mosaic_id: number;
	mosaic_name: string;
	token: string;
	emails: string;
	expires_at: string;
	schedule_start: string | null;
	schedule_end: string | null;
	active: boolean;
	created_at: string | null;
}

export interface ShareAccess {
	mosaic_name: string;
	stream_name: string;
	expires_at: string;
	schedule_start: string | null;
	schedule_end: string | null;
	is_active: boolean;
}

export interface ApiResponse<T> {
	success: boolean;
	data: T;
	error: string | null;
}

export type Tab = 'cameras' | 'mosaics' | 'locations' | 'users' | 'captures' | 'notifications' | 'roles';
