import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import { listen } from '@tauri-apps/api/event';
import { onUnmounted, type Ref, ref, toValue, watch } from 'vue';
import type { MaybeRef } from 'vue';

type RefreshFn = () => Promise<void>;

// --- IconReloadScheduler ---

interface IconEntry {
	id: number;
	refresh: RefreshFn;
	element: HTMLElement | null;
}

interface ReloadController {
	aborted: boolean;
	cancel(): void;
}

function createReloadController(): ReloadController {
	const controller: ReloadController = {
		aborted: false,
		cancel() {
			controller.aborted = true;
		},
	};
	return controller;
}

/**
 * Batched icon reload scheduler.
 * - Debounces theme-change events (100ms): multiple events within 100ms → only the latest processed.
 * - Cancels any in-progress reload cycle when a new one starts.
 * - Splits registered icons into visible (IntersectionObserver) and off-screen.
 * - Reloads visible icons first, then off-screen in batches of 10 with 16ms delay between batches.
 * - On component unmount: removes from registry, discards pending work.
 */
class IconReloadScheduler {
	private entries = new Map<number, IconEntry>();
	private visibleIds = new Set<number>();
	private nextId = 0;
	private observer: IntersectionObserver | null = null;
	private currentCycle: ReloadController | null = null;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;
	private isListening = false;

	private static readonly DEBOUNCE_MS = 100;
	private static readonly BATCH_SIZE = 10;
	private static readonly BATCH_DELAY_MS = 16;

	constructor() {
		this.initObserver();
	}

	private initObserver(): void {
		if (typeof IntersectionObserver === 'undefined') return;
		this.observer = new IntersectionObserver(
			(observerEntries) => {
				for (const observerEntry of observerEntries) {
					const iconId = (observerEntry.target as HTMLElement).dataset?.iconSchedulerId;
					if (iconId == null) continue;
					const id = Number.parseInt(iconId, 10);
					if (observerEntry.isIntersecting) {
						this.visibleIds.add(id);
					} else {
						this.visibleIds.delete(id);
					}
				}
			},
			{ threshold: 0 },
		);
	}

	/**
	 * Register an icon's refresh function and optional DOM element for visibility tracking.
	 * Returns the entry ID (used for unregister).
	 */
	register(refresh: RefreshFn, element?: HTMLElement | null): number {
		const id = this.nextId++;
		const entry: IconEntry = { id, refresh, element: element ?? null };
		this.entries.set(id, entry);

		if (element && this.observer) {
			element.dataset.iconSchedulerId = String(id);
			this.observer.observe(element);
			// Assume visible until observer reports otherwise (covers initial render)
			this.visibleIds.add(id);
		} else {
			// No element → treat as visible (safe default, processes early)
			this.visibleIds.add(id);
		}

		this.ensureListening();
		return id;
	}

	/**
	 * Unregister an icon entry. Removes from visibility tracking and discards pending work.
	 */
	unregister(id: number): void {
		const entry = this.entries.get(id);
		if (!entry) return;

		if (entry.element && this.observer) {
			this.observer.unobserve(entry.element);
			delete entry.element.dataset.iconSchedulerId;
		}

		this.visibleIds.delete(id);
		this.entries.delete(id);
	}

	/**
	 * Called on theme-changed event. Debounces 100ms, then starts batched reload.
	 */
	onThemeChanged(): void {
		// Clear any pending debounce — only latest event matters
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
		}

		this.debounceTimer = setTimeout(() => {
			this.debounceTimer = null;
			this.startReloadCycle();
		}, IconReloadScheduler.DEBOUNCE_MS);
	}

	/**
	 * Cancel any in-progress cycle and start a fresh one.
	 */
	private startReloadCycle(): void {
		// Cancel any in-progress cycle
		if (this.currentCycle) {
			this.currentCycle.cancel();
		}

		const controller = createReloadController();
		this.currentCycle = controller;

		this.executeCycle(controller);
	}

	/**
	 * Execute a reload cycle: visible first, then off-screen in batches.
	 */
	private async executeCycle(controller: ReloadController): Promise<void> {
		// Snapshot current entries so unmounts during cycle are handled
		const allIds = [...this.entries.keys()];
		const visibleIds = allIds.filter((id) => this.visibleIds.has(id));
		const offScreenIds = allIds.filter((id) => !this.visibleIds.has(id));

		// Phase 1: Reload visible icons (all at once or in a batch of BATCH_SIZE)
		const visibleBatches = this.chunk(visibleIds, IconReloadScheduler.BATCH_SIZE);
		for (const batch of visibleBatches) {
			if (controller.aborted) return;
			await this.processBatch(batch, controller);
			// Small delay between visible batches if more than one
			if (visibleBatches.length > 1 && !controller.aborted) {
				await this.delay(IconReloadScheduler.BATCH_DELAY_MS);
			}
		}

		// Phase 2: Reload off-screen icons in batches of 10 with 16ms delay
		const offScreenBatches = this.chunk(offScreenIds, IconReloadScheduler.BATCH_SIZE);
		for (const batch of offScreenBatches) {
			if (controller.aborted) return;
			await this.processBatch(batch, controller);
			if (!controller.aborted) {
				await this.delay(IconReloadScheduler.BATCH_DELAY_MS);
			}
		}

		// Cycle completed normally
		if (this.currentCycle === controller) {
			this.currentCycle = null;
		}
	}

	/**
	 * Process a single batch of icon IDs by calling their refresh functions concurrently.
	 */
	private async processBatch(ids: number[], controller: ReloadController): Promise<void> {
		const promises: Promise<{ id: number } | void>[] = [];
		for (const id of ids) {
			if (controller.aborted) return;
			const entry = this.entries.get(id);
			// Entry may have been unregistered (unmounted) since cycle started
			if (!entry) continue;
			promises.push(
				entry.refresh().catch((err) => {
					console.error(`Icon refresh failed for entry ${id}:`, err);
				}),
			);
		}
		await Promise.allSettled(promises);
	}

	/**
	 * Split an array into chunks of the given size.
	 */
	private chunk(arr: number[], size: number): number[][] {
		const chunks: number[][] = [];
		for (let i = 0; i < arr.length; i += size) {
			chunks.push(arr.slice(i, i + size));
		}
		return chunks;
	}

	/**
	 * Promise-based delay helper.
	 */
	private delay(ms: number): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, ms));
	}

	/**
	 * Set up the Tauri event listener for theme changes (once).
	 */
	private ensureListening(): void {
		if (this.isListening) return;
		this.isListening = true;
		listen('vicons:theme-changed', () => {
			this.onThemeChanged();
		});
	}
}

/** Singleton scheduler instance */
const scheduler = new IconReloadScheduler();

// --- Public API (unchanged signatures) ---

function registerRefresh(fn: RefreshFn, element?: HTMLElement | null): number {
	const entryId = scheduler.register(fn, element ?? null);
	onUnmounted(() => {
		scheduler.unregister(entryId);
	});
	return entryId;
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
	getSource: (name: string) => Promise<string>,
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
			},
		);
	}

	return result;
}

export function useIcons(
	map: Record<string, MaybeRef<string>>,
): Record<string, Ref<string>> {
	return createBatchRefs(map, getIconSource);
}

export function useSymbols(
	map: Record<string, MaybeRef<string>>,
): Record<string, Ref<string>> {
	return createBatchRefs(map, getSymbolSource);
}

export function useReactiveIcons(
	map: Record<string, MaybeRef<string>>,
): Record<string, Ref<string>> {
	return useIcons(map);
}

export function useReactiveSymbols(
	map: Record<string, MaybeRef<string>>,
): Record<string, Ref<string>> {
	return useSymbols(map);
}
