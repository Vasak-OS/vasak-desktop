import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import type { MaybeRef } from 'vue';
import { ref, toValue, watch } from 'vue';
import { useEventListener } from '@/tools/event.listener';

export function useIcon(iconName: MaybeRef<string>) {
	const iconSrc = ref('');
	let requestId = 0;

	async function refresh() {
		const id = ++requestId;
		const result = await getIconSource(toValue(iconName));
		if (id === requestId) iconSrc.value = result;
	}

	watch(() => toValue(iconName), refresh, { immediate: true });
	useEventListener('vicons:theme-changed', refresh);

	return iconSrc;
}

export function useSymbol(iconName: MaybeRef<string>) {
	const iconSrc = ref('');
	let requestId = 0;

	async function refresh() {
		const id = ++requestId;
		const result = await getSymbolSource(toValue(iconName));
		if (id === requestId) iconSrc.value = result;
	}

	watch(() => toValue(iconName), refresh, { immediate: true });
	useEventListener('vicons:theme-changed', refresh);

	return iconSrc;
}

export function useIcons(
	map: Record<string, MaybeRef<string>>
): Record<string, import('vue').Ref<string>> {
	const result: Record<string, import('vue').Ref<string>> = {};
	const keyTokens: Record<string, number> = {};
	let batchId = 0;

	async function refreshAll() {
		const id = ++batchId;
		for (const [key, name] of Object.entries(map)) {
			const src = await getIconSource(toValue(name));
			if (id === batchId) result[key].value = src;
		}
	}

	for (const key of Object.keys(map)) {
		result[key] = ref('');
		keyTokens[key] = 0;
	}

	refreshAll();
	useEventListener('vicons:theme-changed', refreshAll);

	for (const [key, name] of Object.entries(map)) {
		watch(
			() => toValue(name),
			async () => {
				const id = ++keyTokens[key];
				const src = await getIconSource(toValue(name));
				if (id === keyTokens[key]) result[key].value = src;
			}
		);
	}

	return result;
}

export function useSymbols(
	map: Record<string, MaybeRef<string>>
): Record<string, import('vue').Ref<string>> {
	const result: Record<string, import('vue').Ref<string>> = {};
	const keyTokens: Record<string, number> = {};
	let batchId = 0;

	async function refreshAll() {
		const id = ++batchId;
		for (const [key, name] of Object.entries(map)) {
			const src = await getSymbolSource(toValue(name));
			if (id === batchId) result[key].value = src;
		}
	}

	for (const key of Object.keys(map)) {
		result[key] = ref('');
		keyTokens[key] = 0;
	}

	refreshAll();
	useEventListener('vicons:theme-changed', refreshAll);

	for (const [key, name] of Object.entries(map)) {
		watch(
			() => toValue(name),
			async () => {
				const id = ++keyTokens[key];
				const src = await getSymbolSource(toValue(name));
				if (id === keyTokens[key]) result[key].value = src;
			}
		);
	}

	return result;
}
