import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import type { MaybeRef } from 'vue';
import { ref, toValue, watch } from 'vue';
import { useEventListener } from '@/tools/event.listener';

export function useIcon(iconName: MaybeRef<string>) {
	const iconSrc = ref('');

	async function refresh() {
		iconSrc.value = await getIconSource(toValue(iconName));
	}

	watch(() => toValue(iconName), refresh, { immediate: true });
	useEventListener('vicons:theme-changed', refresh);

	return iconSrc;
}

export function useSymbol(iconName: MaybeRef<string>) {
	const iconSrc = ref('');

	async function refresh() {
		iconSrc.value = await getSymbolSource(toValue(iconName));
	}

	watch(() => toValue(iconName), refresh, { immediate: true });
	useEventListener('vicons:theme-changed', refresh);

	return iconSrc;
}

export function useIcons(
	map: Record<string, MaybeRef<string>>
): Record<string, import('vue').Ref<string>> {
	const result: Record<string, import('vue').Ref<string>> = {};

	async function refreshAll() {
		for (const [key, name] of Object.entries(map)) {
			result[key].value = await getIconSource(toValue(name));
		}
	}

	for (const key of Object.keys(map)) {
		result[key] = ref('');
	}

	refreshAll();
	useEventListener('vicons:theme-changed', refreshAll);

	for (const [key, name] of Object.entries(map)) {
		watch(
			() => toValue(name),
			async () => {
				result[key].value = await getIconSource(toValue(name));
			}
		);
	}

	return result;
}

export function useSymbols(
	map: Record<string, MaybeRef<string>>
): Record<string, import('vue').Ref<string>> {
	const result: Record<string, import('vue').Ref<string>> = {};

	async function refreshAll() {
		for (const [key, name] of Object.entries(map)) {
			result[key].value = await getSymbolSource(toValue(name));
		}
	}

	for (const key of Object.keys(map)) {
		result[key] = ref('');
	}

	refreshAll();
	useEventListener('vicons:theme-changed', refreshAll);

	for (const [key, name] of Object.entries(map)) {
		watch(
			() => toValue(name),
			async () => {
				result[key].value = await getSymbolSource(toValue(name));
			}
		);
	}

	return result;
}
