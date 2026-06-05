import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted } from 'vue';

type EventHandler<T> = (event: { payload: T }) => void;

export function useEventListener<T = any>(eventName: string, handler: EventHandler<T>) {
	let unlisten: (() => void) | null = null;
	let mounted = true;

	onMounted(async () => {
		const cleanup = await listen<T>(eventName, handler);
		if (mounted) {
			unlisten = cleanup;
		} else {
			cleanup();
		}
	});

	onUnmounted(() => {
		mounted = false;
		unlisten?.();
	});
}
