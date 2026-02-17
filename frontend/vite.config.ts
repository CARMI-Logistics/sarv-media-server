import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		allowedHosts: ['travis-apivorous-faustina.ngrok-free.dev'],
		proxy: {
			'/api': 'http://localhost:8080',
			'/auth': 'http://localhost:8080',
			'/health': 'http://localhost:8080',
			'/docs': 'http://localhost:8080',
			'/openapi.json': 'http://localhost:8080',
			'/.well-known': 'http://localhost:8080'
		}
	}
});
