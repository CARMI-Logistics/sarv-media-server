import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		allowedHosts: ['travis-apivorous-faustina.ngrok-free.dev'],
		proxy: {
			'/api': 'http://backend-dev:8080',
			'/auth': 'http://backend-dev:8080',
			'/health': 'http://backend-dev:8080',
			'/docs': 'http://backend-dev:8080',
			'/openapi.json': 'http://backend-dev:8080',
			'/.well-known': 'http://backend-dev:8080'
		}
	}
});
