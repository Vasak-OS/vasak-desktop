import { onMounted, onUnmounted } from 'vue';

export function useEscapeToClose(closeFn: () => void) {
	const handler = (event: KeyboardEvent) => {
		if (event.key === 'Escape') {
			closeFn();
		}
	};

	onMounted(() => {
		document.addEventListener('keydown', handler);
	});

	onUnmounted(() => {
		document.removeEventListener('keydown', handler);
	});
}
