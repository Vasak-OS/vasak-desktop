import { invoke } from '@tauri-apps/api/core';

/**
 * Bloquear la pantalla (vasak-desktop#190). El backend le pide a logind que
 * bloquee la sesión (`Session.Lock` por D-Bus) y el bloqueo lo pone quien
 * escucha esa señal: `vasak-lock-screen`, el mismo de la inactividad y de
 * Super+L.
 */
export function lockScreen(): Promise<void> {
	return invoke<void>('lock_screen');
}

/** Falso sin logind o sin sesión gráfica: el botón se ve no disponible. */
export function isLockScreenAvailable(): Promise<boolean> {
	return invoke<boolean>('lock_screen_available');
}
