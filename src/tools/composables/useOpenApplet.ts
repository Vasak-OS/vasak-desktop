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

/**
 * Las clases del botón cuyo applet está abierto: así se lee de dónde salió.
 *
 * Es lo **elegido**, y lo elegido va con el velo del acento
 * (`ui-selected-accent`, decisión 4 de vue-libvasak#74), no con el relleno
 * entero del primario: un botón de 30 píxeles pintado de rosa en el panel era
 * lo más llamativo de la pantalla, y lo que se abrió es el applet, no el botón.
 * El texto y el icono siguen con su color, que sobre el velo sigue pasando
 * 4,5:1.
 */
export const OPEN_APPLET_CLASSES = 'bg-ui-selected-accent';

export interface OpenAppletState {
	isOpen: ComputedRef<boolean>;
	openClasses: ComputedRef<Record<string, boolean>>;
}

/**
 * Lo que un botón necesita saber de su applet, sin suscribirse a nada.
 *
 * Aparte de `useOpenApplet` porque la suscripción necesita un componente
 * montado, y esto es lo que se puede probar sin montar uno.
 */
export function openAppletState(applet: AppletId): OpenAppletState {
	const isOpen = computed(() => openApplet.value === applet);

	return {
		isOpen,
		openClasses: computed(() => ({ [OPEN_APPLET_CLASSES]: isOpen.value })),
	};
}

export function useOpenApplet(applet: AppletId): OpenAppletState {
	useSharedEvent<{ applet: string | null }>('applet-changed', applyAppletChanged);
	return openAppletState(applet);
}

/**
 * Si hay **algún** applet abierto, el que sea.
 *
 * Lo usa el auto-ocultar para no esconder la barra con un applet colgando de
 * ella: el applet es su propia superficie, pero esconder el panel que lo abrió
 * mientras se lo usa es desconcertante.
 */
export function useAnyAppletOpen(): ComputedRef<boolean> {
	useSharedEvent<{ applet: string | null }>('applet-changed', applyAppletChanged);
	return computed(() => openApplet.value !== null);
}

/** Para las pruebas: el valor compartido, sin suscribirse a nada. */
export const openAppletForTests = openApplet;
