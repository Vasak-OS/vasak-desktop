/**
 * Limita cuántas veces por segundo se llama a `send`, sin perder el último.
 *
 * Es para arrastrar una banda del ecualizador: el servicio pide `SetGain` como
 * mucho unas 30 veces por segundo, pero lo que importa es que **el último**
 * valor llegue. La primera llamada sale en el acto; las que llegan antes de que
 * pase `interval` se juntan, y al cumplirse sale sólo la más nueva. Cada clave
 * (cada banda) lleva su propia cuenta.
 *
 * `clock` y `schedule` se pueden cambiar, para probarlo sin esperar.
 */
export interface RateLimiter<K, V> {
	push(key: K, value: V): void;
	/** Manda lo pendiente ya, y suelta los temporizadores. */
	flush(): void;
}

export function rateLimited<K, V>(
	send: (key: K, value: V) => void,
	interval: number,
	clock: () => number = () => Date.now(),
	schedule: (fn: () => void, ms: number) => unknown = (fn, ms) => setTimeout(fn, ms),
	cancel: (handle: unknown) => void = (handle) =>
		clearTimeout(handle as ReturnType<typeof setTimeout>)
): RateLimiter<K, V> {
	const lastSent = new Map<K, number>();
	const pending = new Map<K, V>();
	const timers = new Map<K, unknown>();

	function fire(key: K): void {
		timers.delete(key);
		if (!pending.has(key)) return;
		const value = pending.get(key) as V;
		pending.delete(key);
		lastSent.set(key, clock());
		send(key, value);
	}

	return {
		push(key, value) {
			const since = clock() - (lastSent.get(key) ?? Number.NEGATIVE_INFINITY);
			pending.set(key, value);
			if (since >= interval && !timers.has(key)) {
				fire(key);
				return;
			}
			if (!timers.has(key))
				timers.set(
					key,
					schedule(() => fire(key), Math.max(interval - since, 0))
				);
		},
		flush() {
			for (const [key, handle] of timers) {
				cancel(handle);
				timers.delete(key);
				fire(key);
			}
		},
	};
}
