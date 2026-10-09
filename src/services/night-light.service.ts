import { invoke } from '@tauri-apps/api/core';

/**
 * La luz nocturna (vasak-desktop#178). `wlsunset` corre como la unidad de
 * usuario `vasak-nightlight.service`, que el escritorio prende y apaga por el
 * D-Bus de systemd. La configuración (temperatura y horario) es de
 * `@vasakgroup/plugin-display-manager`, compartida con Configuración.
 */
export interface NightLightState {
	/** Falso sin `wlsunset` instalado. */
	available: boolean;
	enabled: boolean;
}

/**
 * El evento que llega a cada ventana cuando la luz cambia. El escritorio la
 * sigue sólo con el centro de control abierto: un cambio hecho con el centro
 * cerrado llega al abrirlo.
 */
export const NIGHT_LIGHT_EVENT = 'night-light-changed';

export function getNightLightState(): Promise<NightLightState> {
	return invoke<NightLightState>('get_night_light_state');
}

/** La prende o la apaga y devuelve cómo estaba. Rechaza con el motivo si no pudo. */
export function setNightLightEnabled(enabled: boolean): Promise<boolean> {
	return invoke<boolean>('set_night_light_enabled', { enabled });
}

/**
 * Aplica la configuración recién guardada con el plugin: si la luz está
 * encendida la reinicia, porque `wlsunset` lee sus valores al arrancar.
 * Devuelve si hubo que reiniciarla.
 */
export function applyNightLight(): Promise<boolean> {
	return invoke<boolean>('apply_night_light');
}
