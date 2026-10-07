import { invoke } from '@tauri-apps/api/core';

/**
 * Modo avión (vasak-desktop#180). Lo sostiene el escritorio leyendo los eventos
 * de `/dev/rfkill`: leerlo devuelve la copia en memoria, sin tocar el
 * dispositivo.
 */
export interface AirplaneModeState {
	/** Falso sin radios, o sin permiso para bloquearlas. */
	available: boolean;
	/** Hay radios y están todas bloqueadas. */
	enabled: boolean;
	/** Alguna está bloqueada por el equipo: quitar el modo no la prende. */
	hardware: boolean;
	/** Hay Wi-Fi y está todo bloqueado. */
	wlanBlocked: boolean;
	/** Hay Bluetooth y está todo bloqueado. */
	bluetoothBlocked: boolean;
}

/** El evento que llega a cada ventana con cada cambio, venga de donde venga. */
export const AIRPLANE_MODE_EVENT = 'airplane-mode-changed';

export function getAirplaneMode(): Promise<AirplaneModeState> {
	return invoke<AirplaneModeState>('get_airplane_mode');
}

/** Pone o quita el modo y devuelve el que había. */
export function setAirplaneMode(enabled: boolean): Promise<boolean> {
	return invoke<boolean>('set_airplane_mode', { enabled });
}

/**
 * Desbloquea las radios de un tipo. Lo usan los mosaicos de Wi-Fi y Bluetooth
 * al tocarlos con la radio bloqueada, sin depender de que NetworkManager o
 * BlueZ sepan sacar el bloqueo.
 */
export function unblockRadios(kind: 'wlan' | 'bluetooth'): Promise<void> {
	return invoke<void>('unblock_radios', { kind });
}
