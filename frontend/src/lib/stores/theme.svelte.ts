export type ThemeMode = 'light' | 'dark';

class ThemeStore {
	mode = $state<ThemeMode>('dark');

	constructor() {
		if (typeof window !== 'undefined') {
			const saved = localStorage.getItem('theme') as ThemeMode | null;
			if (saved === 'light' || saved === 'dark') {
				this.mode = saved;
			}
		}
	}

	get isDark() { return this.mode === 'dark'; }

	toggle() {
		this.mode = this.mode === 'dark' ? 'light' : 'dark';
		if (typeof window !== 'undefined') {
			localStorage.setItem('theme', this.mode);
		}
		this.apply();
	}

	apply() {
		if (typeof document !== 'undefined') {
			document.documentElement.classList.toggle('dark', this.mode === 'dark');
		}
	}
}

export const theme = new ThemeStore();
