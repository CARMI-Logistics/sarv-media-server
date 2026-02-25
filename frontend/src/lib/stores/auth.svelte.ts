import { goto } from '$app/navigation';

class AuthStore {
	token = $state<string | null>(null);

	constructor() {
		// Token is now stored in httpOnly cookie, not localStorage
		// We'll get it from API responses if needed
		if (typeof window !== 'undefined') {
			// Check if we have a token in cookie by trying to read it
			// (we can't directly access httpOnly cookies from JS, but the server will send it)
			this.checkAuth();
		}
	}

	async checkAuth() {
		// Verify if we're authenticated by checking if cookie exists
		// This is done automatically by the browser sending the cookie
		try {
			const response = await fetch('/health', {
				credentials: 'include' // Include cookies in request
			});
			// If we can access protected endpoints, we're authenticated
			// The actual token validation happens server-side
		} catch (e) {
			// Not authenticated or server error
		}
	}

	setToken(t: string) {
		// Token is now set via httpOnly cookie by the server
		// We store it only for backwards compatibility with API calls
		this.token = t;
		// NO localStorage - cookie is handled by browser automatically
	}

	async logout() {
		this.token = null;
		// Clear the httpOnly cookie by calling logout endpoint
		try {
			await fetch('/auth/logout', {
				method: 'POST',
				credentials: 'include'
			});
		} catch (e) {
			console.error('Logout error:', e);
		}
		goto('/login');
	}

	get isAuthenticated(): boolean {
		// Check if we have a token (for backwards compatibility)
		// In production, authentication is verified server-side via cookie
		return this.token !== null;
	}
}

export const auth = new AuthStore();
