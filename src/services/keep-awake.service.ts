import { invoke } from '@tauri-apps/api/core';

/**
 * «Mantener despierto» (vasak-desktop#179). Mientras está puesto, el escritorio
 * tiene tomado un inhibidor de logind: ni se bloquea la pantalla ni se suspende
 * por inactividad. No se recuerda entre sesiones.
 */
export interface KeepAwakeState {
	/** Falso sin logind en el bus del sistema. */
	available: boolean;
	enabled: boolean;
}

/** El evento que llega a cada ventana cuando el modo cambia. */
export const KEEP_AWAKE_EVENT = 'keep-awake-changed';

export function getKeepAwake(): Promise<KeepAwakeState> {
	return invoke<KeepAwakeState>('get_keep_awake');
}

/** Lo pone o lo quita y devuelve si estaba puesto. */
export function setKeepAwake(enabled: boolean): Promise<boolean> {
	return invoke<boolean>('set_keep_awake', { enabled });
}
