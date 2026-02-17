import { auth } from '$lib/stores/auth.svelte';
import type { ApiResponse } from '$lib/types';

// Prioridad: 1) VITE_API_URL (variable de entorno)
//             2) DEV mode -> localhost:8080
//             3) Producción -> mismo origen
const API = import.meta.env.VITE_API_URL || (import.meta.env.DEV ? 'http://localhost:8080' : '');

console.log('[API] Using backend URL:', API || '(same origin)');

export async function authFetch(url: string, opts: RequestInit = {}): Promise<Response> {
	const token = auth.token;
	if (!token) {
		auth.logout();
		throw new Error('No token');
	}
	opts.headers = { ...opts.headers as Record<string, string>, Authorization: `Bearer ${token}` };
	const res = await fetch(url, opts);
	if (res.status === 401) {
		auth.logout();
		throw new Error('Unauthorized');
	}
	return res;
}

export async function apiGet<T>(path: string): Promise<ApiResponse<T>> {
	const res = await authFetch(`${API}${path}`);
	return res.json();
}

export async function apiPost<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
	const res = await authFetch(`${API}${path}`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: body ? JSON.stringify(body) : undefined
	});
	return res.json();
}

export async function apiPut<T>(path: string, body: unknown): Promise<ApiResponse<T>> {
	const res = await authFetch(`${API}${path}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(body)
	});
	return res.json();
}

export async function apiDelete<T>(path: string): Promise<ApiResponse<T>> {
	const res = await authFetch(`${API}${path}`, { method: 'DELETE' });
	return res.json();
}

export async function login(username: string, password: string): Promise<{ token?: string; error?: string }> {
	try {
		const res = await fetch(`${API}/auth/login`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ username, password })
		});
		const json = await res.json();
		if (res.ok && json.token) return { token: json.token };
		return { error: json.error || 'Credenciales inválidas' };
	} catch {
		return { error: 'Error de conexión con el servidor' };
	}
}
