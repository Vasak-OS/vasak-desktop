import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import { listen } from '@tauri-apps/api/event';
import { onUnmounted, type Ref, ref, toValue, watch } from 'vue';
import type { MaybeRef } from 'vue';

type RefreshFn = () => Promise<void>;

let isListening = false;
const refreshFns = new Set<RefreshFn>();

async function ensureListening() {
	if (isListening) return;
	isListening = true;
	await listen('vicons:theme-changed', () => {
		for (const fn of refreshFns) {
			fn();
		}
	});
}

function registerRefresh(fn: RefreshFn) {
	refreshFns.add(fn);
	ensureListening();
	onUnmounted(() => {
		refreshFns.delete(fn);
	});
}

export function useIcon(iconName: MaybeRef<string>): Ref<string> {
	const iconSrc = ref('');
	let requestId = 0;

	const refresh: RefreshFn = async () => {
		const id = ++requestId;
		const name = toValue(iconName);
		if (!name) {
			if (id === requestId) iconSrc.value = '';
			return;
		}
		const result = await getIconSource(name);
		if (id === requestId) iconSrc.value = result;
	};

	watch(() => toValue(iconName), refresh, { immediate: true });
	registerRefresh(refresh);

	return iconSrc;
}

export function useSymbol(iconName: MaybeRef<string>): Ref<string> {
	const iconSrc = ref('');
	let requestId = 0;

	const refresh: RefreshFn = async () => {
		const id = ++requestId;
		const name = toValue(iconName);
		if (!name) {
			if (id === requestId) iconSrc.value = '';
			return;
		}
		const result = await getSymbolSource(name);
		if (id === requestId) iconSrc.value = result;
	};

	watch(() => toValue(iconName), refresh, { immediate: true });
	registerRefresh(refresh);

	return iconSrc;
}

export function useReactiveIcon(iconName: MaybeRef<string>): Ref<string> {
	return useIcon(iconName);
}

export function useReactiveSymbol(iconName: MaybeRef<string>): Ref<string> {
	return useSymbol(iconName);
}

function createBatchRefs(
	map: Record<string, MaybeRef<string>>,
	getSource: (name: string) => Promise<string>
): Record<string, Ref<string>> {
	const result: Record<string, Ref<string>> = {};
	const keyTokens: Record<string, number> = {};

	const refreshAll: RefreshFn = async () => {
		for (const [key, name] of Object.entries(map)) {
			const keyId = ++keyTokens[key];
			const src = await getSource(toValue(name));
			if (keyId === keyTokens[key]) result[key].value = src;
		}
	};

	for (const key of Object.keys(map)) {
		result[key] = ref('');
		keyTokens[key] = 0;
	}

	refreshAll();
	registerRefresh(refreshAll);

	for (const [key, name] of Object.entries(map)) {
		watch(
			() => toValue(name),
			async () => {
				const keyId = ++keyTokens[key];
				const src = await getSource(toValue(name));
				if (keyId === keyTokens[key]) result[key].value = src;
			}
		);
	}

	return result;
}

export function useIcons(
	map: Record<string, MaybeRef<string>>
): Record<string, Ref<string>> {
	return createBatchRefs(map, getIconSource);
}

export function useSymbols(
	map: Record<string, MaybeRef<string>>
): Record<string, Ref<string>> {
	return createBatchRefs(map, getSymbolSource);
}

export function useReactiveIcons(
	map: Record<string, MaybeRef<string>>
): Record<string, Ref<string>> {
	return useIcons(map);
}

export function useReactiveSymbols(
	map: Record<string, MaybeRef<string>>
): Record<string, Ref<string>> {
	return useSymbols(map);
}
