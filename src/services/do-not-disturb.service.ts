import { invoke } from '@tauri-apps/api/core';

/**
 * «No molestar» (vasak-desktop#177). El modo vive en el demonio de
 * notificaciones; el escritorio guarda una copia en memoria que se actualiza
 * por señales, así que leerlo no cruza el bus.
 */
export interface DoNotDisturbState {
	/** Falso sin demonio de notificaciones, o con uno anterior a la 0.6.0. */
	available: boolean;
	enabled: boolean;
}

/** El evento que llega a cada ventana cuando el modo cambia, venga de donde venga. */
export const DO_NOT_DISTURB_EVENT = 'do-not-disturb-changed';

export function getDoNotDisturb(): Promise<DoNotDisturbState> {
	return invoke<DoNotDisturbState>('get_do_not_disturb');
}

/**
 * Pone el modo y devuelve el que había, para poder dejarlo como estaba:
 *
 * ```ts
 * const previous = await setDoNotDisturb(true);
 * // …
 * await setDoNotDisturb(previous);
 * ```
 */
export function setDoNotDisturb(enabled: boolean): Promise<boolean> {
	return invoke<boolean>('set_do_not_disturb', { enabled });
}
