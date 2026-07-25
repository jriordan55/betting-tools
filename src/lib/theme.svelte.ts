/** Theme state. The web app is dark-only; the desktop app gets both. */

export type Theme = 'dark' | 'light';

const STORAGE_KEY = 'bettor-theme';

function initial(): Theme {
	if (typeof localStorage !== 'undefined') {
		const saved = localStorage.getItem(STORAGE_KEY);
		if (saved === 'dark' || saved === 'light') return saved;
	}
	if (typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: light)').matches) {
		return 'light';
	}
	return 'dark';
}

class ThemeState {
	current = $state<Theme>('dark');

	load() {
		this.current = initial();
		this.apply();
	}

	toggle() {
		this.current = this.current === 'dark' ? 'light' : 'dark';
		localStorage.setItem(STORAGE_KEY, this.current);
		this.apply();
	}

	private apply() {
		document.documentElement.setAttribute('data-theme', this.current);
	}
}

export const theme = new ThemeState();
