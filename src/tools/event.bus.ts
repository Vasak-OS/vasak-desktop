import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted } from 'vue';

export interface EventBusOptions {
	throttleMs?: number; // Max emission rate (e.g., 16ms for 60fps)
	debounceMs?: number; // Trailing-edge debounce (e.g., 150ms)
}

interface ListenerEntry {
	unlisten: (() => void) | null;
	handlers: Set<(payload: unknown) => void>;
	options: EventBusOptions;
	lastEmitTime: number;
	throttleTimer: ReturnType<typeof setTimeout> | null;
	throttlePending: unknown;
	debounceTimer: ReturnType<typeof setTimeout> | null;
}

class SharedEventBus {
	private readonly listeners: Map<string, ListenerEntry> = new Map();

	/**
	 * Subscribe to a Tauri event with optional throttle/debounce.
	 * Returns an unsubscribe function.
	 */
	subscribe<T>(
		event: string,
		handler: (payload: T) => void,
		options?: EventBusOptions,
	): () => void {
		const entry = this.listeners.get(event);

		if (entry) {
			if (options && (options.throttleMs !== entry.options.throttleMs || options.debounceMs !== entry.options.debounceMs)) {
				console.warn(
					`[SharedEventBus] Handler for "${event}" registered with conflicting timing options. ` +
					`Using first subscriber's options (throttleMs=${entry.options.throttleMs}, debounceMs=${entry.options.debounceMs}).`,
				);
			}
			entry.handlers.add(handler as unknown as (payload: unknown) => void);
		} else {
			const newEntry: ListenerEntry = {
				unlisten: null,
				handlers: new Set([handler as unknown as (payload: unknown) => void]),
				options: options ?? {},
				lastEmitTime: 0,
				throttleTimer: null,
				throttlePending: null as unknown,
				debounceTimer: null,
			};
			this.listeners.set(event, newEntry);
			this.ensureListener(event, options ?? {});
		}

		return () => this.unsubscribe(event, handler as unknown as (payload: unknown) => void);
	}

	/**
	 * Creates a single Tauri listen() per unique event name.
	 * Handles throttle/debounce before fanning out to handlers.
	 */
	private ensureListener(event: string, _options: EventBusOptions): void {
		const entry = this.listeners.get(event);
		if (entry?.unlisten != null) return;
		if (!entry) return;

		const registerListener = async (isRetry = false) => {
			try {
				const unlisten = await listen(event, (tauriEvent) => {
					this.dispatch(event, tauriEvent.payload);
				});

				const currentEntry = this.listeners.get(event);
				if (currentEntry === entry) {
					entry.unlisten = unlisten;
				} else {
					// The original entry was removed (and possibly replaced).
					// Stale registration has no business assigning to the current entry.
					unlisten();
				}
			} catch (error) {
				console.error(
					`[SharedEventBus] Failed to register listener for "${event}"${isRetry ? ' (retry)' : ''}:`,
					error,
				);

				if (!isRetry) {
					// Retry once within 1 second
					setTimeout(() => {
						const retryEntry = this.listeners.get(event);
						if (retryEntry === entry && entry.unlisten === null && entry.handlers.size > 0) {
							registerListener(true);
						}
					}, 1000);
				} else {
					// Notify subscribers of failure after retry exhausted
					const failEntry = this.listeners.get(event);
					if (failEntry === entry && failEntry.handlers.size > 0) {
						console.error(
							`[SharedEventBus] Listener registration failed for "${event}" after retry. ` +
								`${failEntry.handlers.size} subscriber(s) will not receive events.`,
						);
					}
				}
			}
		};

		registerListener();
	}

	/**
	 * Dispatch payload to all handlers with throttle/debounce logic.
	 */
	private dispatch(event: string, payload: unknown): void {
		const entry = this.listeners.get(event);
		if (!entry) return;

		const { options } = entry;

		if (options.throttleMs) {
			this.dispatchThrottled(entry, payload);
		} else if (options.debounceMs) {
			this.dispatchDebounced(entry, payload);
		} else {
			this.fanout(entry, payload);
		}
	}

	/**
	 * Throttle: last-value-wins within the throttle window.
	 * Delivers immediately if enough time has passed, otherwise schedules
	 * delivery of the latest value at the end of the window.
	 */
	private dispatchThrottled(entry: ListenerEntry, payload: unknown): void {
		const now = Date.now();
		const throttleMs = entry.options.throttleMs!;
		const elapsed = now - entry.lastEmitTime;

		if (elapsed >= throttleMs) {
			// Enough time has passed, deliver immediately
			entry.lastEmitTime = now;
			this.fanout(entry, payload);
		} else {
			// Store latest value; schedule delivery at end of window
			entry.throttlePending = payload;
			entry.throttleTimer ??= setTimeout(() => {
				entry.throttleTimer = null;
				entry.lastEmitTime = Date.now();
				const pending = entry.throttlePending;
				entry.throttlePending = null;
				this.fanout(entry, pending);
			}, throttleMs - elapsed);
		}
	}

	/**
	 * Debounce: trailing-edge. Delivers only after silence of debounceMs.
	 * Always delivers the latest payload.
	 */
	private dispatchDebounced(entry: ListenerEntry, payload: unknown): void {
		const debounceMs = entry.options.debounceMs!;

		if (entry.debounceTimer !== null) {
			clearTimeout(entry.debounceTimer);
		}

		entry.debounceTimer = setTimeout(() => {
			entry.debounceTimer = null;
			this.fanout(entry, payload);
		}, debounceMs);
	}

	/**
	 * Fan out the payload to all registered handlers.
	 */
	private fanout(entry: ListenerEntry, payload: unknown): void {
		for (const handler of entry.handlers) {
			try {
				handler(payload);
			} catch (error) {
				console.error('[SharedEventBus] Handler threw an error:', error);
			}
		}
	}

	/**
	 * Removes a handler. When subscriber count reaches zero,
	 * unregisters the Tauri listener and cleans up timers.
	 */
	private unsubscribe(event: string, handler: (payload: unknown) => void): void {
		const entry = this.listeners.get(event);
		if (!entry) return;

		entry.handlers.delete(handler);

		if (entry.handlers.size === 0) {
			// Clean up timers
			if (entry.throttleTimer !== null) {
				clearTimeout(entry.throttleTimer);
				entry.throttleTimer = null;
			}
			if (entry.debounceTimer !== null) {
				clearTimeout(entry.debounceTimer);
				entry.debounceTimer = null;
			}

			// Unregister Tauri listener
			if (entry.unlisten) {
				entry.unlisten();
			}

			this.listeners.delete(event);
		}
	}
}

/** Singleton shared event bus instance */
export const eventBus = new SharedEventBus();

/**
 * Vue composable that auto-subscribes on mount and unsubscribes on unmount.
 * Uses the shared event bus to deduplicate Tauri listeners.
 */
export function useSharedEvent<T>(
	event: string,
	handler: (payload: T) => void,
	options?: EventBusOptions,
): void {
	let unsubscribe: (() => void) | null = null;

	onMounted(() => {
		unsubscribe = eventBus.subscribe<T>(event, handler, options);
	});

	onUnmounted(() => {
		unsubscribe?.();
		unsubscribe = null;
	});
}
