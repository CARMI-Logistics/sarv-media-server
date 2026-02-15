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

export interface ApiResponse<T> {
	success: boolean;
	data: T;
	error: string | null;
}

export type Tab = 'cameras' | 'mosaics' | 'locations' | 'areas';
