import { onBeforeUnmount, type Ref, ref, watch } from 'vue';
import { useAnyAppletOpen } from '@/tools/composables/useOpenApplet';
import { isNearInnerEdge } from '@/tools/panel-autohide';
import type { PanelPosition } from '@/tools/panel-position';

/**
 * El estado de esconder y revelar la barra cuando `panel.autohide` está puesto.
 *
 * Se maneja con lo único que este WebView entrega de forma fiable sobre una
 * superficie de capa: los `pointermove` **con coordenadas** mientras el puntero
 * está sobre la franja. No llega ningún evento de salida del puntero, así que el
 * «salió» se deduce de la posición: cuando el puntero llega al **borde interior**
 * de la franja —el que da al contenido, por donde se sale— se esconde pronto; si
 * se mueve por el medio, apuntando a algo, se queda. Con la barra escondida la
 * región de entrada es sólo la línea del borde, así que cualquier `pointermove`
 * viene de ahí y revela.
 *
 * El backend también reenvía los cruces de GTK (`panel-pointer-entered` /
 * `panel-pointer-left`) por si en algún entorno sí llegan: la vista los conecta a
 * `reveal` / `requestHide`. Y hay una red de seguridad por inactividad larga, por
 * si nada de eso disparara, para que la barra no quede abierta para siempre.
 *
 * El backend ya dejó la zona exclusiva en cero (`panel_autohide.rs`): esto es
 * sólo la conducta de la interfaz. Aparte de la vista para poder probar su
 * máquina de estados sin montar Vue.
 */

/** Tras llegar el puntero al borde interior (saliendo), lo que se espera antes de esconder. */
export const HIDE_DELAY_MS = 300;

/**
 * Red de seguridad: sin actividad del puntero por este tiempo —p. ej. salió de un
 * tirón sin rozar el borde interior— se esconde igual. Más largo que la espera de
 * salida para no esconder mientras alguien apunta a algo quieto en el medio.
 */
export const IDLE_SAFETY_MS = 2500;

export interface PanelAutohide {
	/** `true` cuando la barra está escondida (deslizada fuera de la pantalla). */
	hidden: Ref<boolean>;
	/** Trae la barra a la vista (el puntero entró en la franja). */
	reveal: () => void;
	/** Agenda esconderla (el puntero salió), salvo que haya un applet abierto. */
	requestHide: () => void;
}

/**
 * `appletOpen` se inyecta para poder probar la máquina de estados sin el bus de
 * eventos de Tauri: por omisión es el estado real (`useAnyAppletOpen`).
 */
export function usePanelAutohide(
	autohide: Ref<boolean>,
	position: Ref<PanelPosition>,
	appletOpen: Ref<boolean> = useAnyAppletOpen()
): PanelAutohide {
	const hidden = ref(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	const clearTimer = () => {
		if (timer) {
			clearTimeout(timer);
			timer = undefined;
		}
	};

	/** Agenda esconder dentro de `delay`, salvo que haya un applet abierto. */
	const armHide = (delay: number) => {
		clearTimer();
		if (!autohide.value || appletOpen.value) return;
		timer = setTimeout(() => {
			hidden.value = true;
		}, delay);
	};

	const reveal = () => {
		hidden.value = false;
		armHide(IDLE_SAFETY_MS);
	};

	const requestHide = () => armHide(HIDE_DELAY_MS);

	/**
	 * Llega `pointermove` al documento. Con la barra escondida la región de entrada
	 * es sólo la línea del borde, así que esto revela. Y, por la posición, decide:
	 * contra el borde interior (saliendo) → esconder pronto; por el medio
	 * (apuntando) → sólo la red de seguridad, se queda.
	 */
	const onPointerMove = (event: PointerEvent) => {
		if (hidden.value) hidden.value = false;
		const leaving =
			typeof window !== 'undefined' &&
			isNearInnerEdge(
				event.clientX,
				event.clientY,
				position.value,
				window.innerWidth,
				window.innerHeight
			);
		armHide(leaving ? HIDE_DELAY_MS : IDLE_SAFETY_MS);
	};

	const bindDocument = (on: boolean) => {
		if (typeof document === 'undefined') return;
		if (on) {
			document.addEventListener('pointermove', onPointerMove);
			document.addEventListener('pointerdown', onPointerMove);
		} else {
			document.removeEventListener('pointermove', onPointerMove);
			document.removeEventListener('pointerdown', onPointerMove);
		}
	};

	// Prender el auto-ocultar arranca la barra escondida y se pone a escuchar el
	// documento; apagarlo la devuelve a la vista y suelta todo.
	watch(
		autohide,
		(on) => {
			clearTimer();
			bindDocument(false);
			if (on) {
				hidden.value = true;
				bindDocument(true);
			} else {
				hidden.value = false;
			}
		},
		{ immediate: true }
	);

	// Con un applet abierto la barra se queda a la vista; al cerrarse el último,
	// se agenda esconder como si el puntero hubiera salido.
	watch(appletOpen, (open) => {
		if (open) reveal();
		else requestHide();
	});

	onBeforeUnmount(() => {
		clearTimer();
		bindDocument(false);
	});

	return { hidden, reveal, requestHide };
}
