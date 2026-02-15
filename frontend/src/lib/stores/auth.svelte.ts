import { goto } from '$app/navigation';

class AuthStore {
	token = $state<string | null>(null);
	isAuthenticated = $derived(!!this.token);

	constructor() {
		if (typeof window !== 'undefined') {
			this.token = localStorage.getItem('jwt_token');
		}
	}

	setToken(t: string) {
		this.token = t;
		localStorage.setItem('jwt_token', t);
	}

	logout() {
		this.token = null;
		localStorage.removeItem('jwt_token');
		goto('/login');
	}
}

export const auth = new AuthStore();
