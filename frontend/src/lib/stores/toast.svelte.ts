export type ToastType = 'success' | 'error' | 'info';

interface ToastItem {
	id: number;
	message: string;
	type: ToastType;
}

class ToastStore {
	items = $state<ToastItem[]>([]);
	private nextId = 0;

	show(message: string, type: ToastType = 'info') {
		const id = this.nextId++;
		this.items.push({ id, message, type });
		setTimeout(() => this.dismiss(id), 3500);
	}

	dismiss(id: number) {
		this.items = this.items.filter((t) => t.id !== id);
	}

	success(msg: string) { this.show(msg, 'success'); }
	error(msg: string) { this.show(msg, 'error'); }
	info(msg: string) { this.show(msg, 'info'); }
}

export const toast = new ToastStore();
