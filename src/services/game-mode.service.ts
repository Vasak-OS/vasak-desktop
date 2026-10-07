import { invoke } from '@tauri-apps/api/core';

/**
 * El modo juego (vasak-desktop#181). Lo sostiene el escritorio: al entrar
 * aplica las acciones que diga `vasak.conf` (No molestar, sin animaciones de
 * Wayfire, `gamemoded`) y al salir deja todo como estaba. No se recuerda entre
 * sesiones.
 */

/** El evento que llega a cada ventana cuando el modo cambia; trae el estado nuevo. */
export const GAME_MODE_EVENT = 'game-mode-changed';

export function getGameMode(): Promise<boolean> {
	return invoke<boolean>('get_game_mode');
}

/** Pone el modo y devuelve el que había. Rechaza con el motivo si no pudo. */
export function setGameMode(enabled: boolean): Promise<boolean> {
	return invoke<boolean>('set_game_mode', { enabled });
}
