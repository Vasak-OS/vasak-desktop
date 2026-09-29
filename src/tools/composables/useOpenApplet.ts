import { type ComputedRef, computed, ref } from 'vue';
import type { AppletId } from '@/services/window.service';
import { useSharedEvent } from '@/tools/event.bus';

/**
 * Qué applet está abierto, según el backend.
 *
 * El backend avisa con `applet-changed` cada vez que abre o cierra uno, lo
 * cierre quien lo cierre: un clic en el panel, Escape, la pérdida de foco o la
 * propia página. El panel no lo adivina porque la mitad de esos cierres no los
 * ve.
 *
 * Una sola referencia para todo el panel: cada botón se suscribe, pero el valor
 * es el mismo para todos.
 */
const openApplet = ref<string | null>(null);

/** Aplica un aviso de `applet-changed`. Aparte para poder probarlo. */
export function applyAppletChanged(payload: { applet?: string | null } | null | undefined): void {
	openApplet.value = payload?.applet ?? null;
}

/** Las clases del botón cuyo applet está abierto: así se lee de dónde salió. */
export const OPEN_APPLET_CLASSES = 'bg-primary text-tx-on-primary';

export function useOpenApplet(applet: AppletId): {
	isOpen: ComputedRef<boolean>;
	openClasses: ComputedRef<Record<string, boolean>>;
} {
	useSharedEvent<{ applet: string | null }>('applet-changed', applyAppletChanged);

	const isOpen = computed(() => openApplet.value === applet);

	return {
		isOpen,
		openClasses: computed(() => ({ [OPEN_APPLET_CLASSES]: isOpen.value })),
	};
}

/** Para las pruebas: el valor compartido, sin suscribirse a nada. */
export const openAppletForTests = openApplet;
