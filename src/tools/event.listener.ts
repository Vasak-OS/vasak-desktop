import { useSharedEvent } from '@/tools/event.bus';

type EventHandler<T> = (event: { payload: T }) => void;

/**
 * @deprecated Use `useSharedEvent` from `@/tools/event.bus` instead.
 * This wrapper delegates to the shared event bus for backward compatibility.
 * The handler signature wraps the payload in `{ payload: T }` to match the legacy API.
 */
export function useEventListener<T = any>(eventName: string, handler: EventHandler<T>) {
	useSharedEvent<T>(eventName, (payload) => {
		handler({ payload });
	});
}
