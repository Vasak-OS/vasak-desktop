/**
 * Los dobles de lo que importa `SystemResourceControl.vue` desde `@/…`
 * (vasak-desktop#216). La cuenta de CPU y de memoria es la de verdad; el bus de
 * eventos es un doble que la prueba dispara con `resources.emit`.
 */
export { cpuPercent, memoryPercent, type SystemStats } from '../../src/tools/system-stats';

type Handler = (payload: unknown) => void;

export const resources = {
	handlers: new Map<string, Handler>(),
	emit(name: string, payload: unknown) {
		this.handlers.get(name)?.(payload);
	},
	reset() {
		this.handlers.clear();
	},
};

export function useSharedEvent(name: string, handler: Handler) {
	resources.handlers.set(name, handler);
}

export const logError = () => {};
