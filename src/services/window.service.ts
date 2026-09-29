import { invoke } from '@tauri-apps/api/core';
import { anchorOf } from '@/tools/applet-anchor';

/** Los applets del panel. Son las filas de `APPLETS` en `anchored_applet.rs`. */
export type AppletId = 'bluetooth' | 'network' | 'audio' | 'tray' | 'privacy' | 'twingate';

/**
 * Abre o cierra un applet, colgado del botón que lo pidió.
 *
 * `button` es el elemento del botón —o el componente, tal como lo da un `ref`—:
 * el backend lo usa para ubicar el applet debajo. Sin botón, el applet se
 * centra en el eje del panel.
 */
export const toggleApplet = (applet: AppletId, button?: unknown): Promise<void> => {
	return invoke<void>('toggle_applet', { applet, anchor: anchorOf(button) ?? null });
};

/**
 * Cierra con su animación el applet `applet`, si es el que está abierto.
 *
 * Para cuando el applet termina lo que vino a hacer. No sirve el conmutador:
 * esconder saca el foco, perder el foco cierra, y el conmutador lo encontraría
 * cerrado y lo volvería a abrir. Y se nombra el applet porque el pedido de una
 * página que se estaba yendo puede llegar cuando ya se abrió otro.
 */
export const dismissApplet = (applet: AppletId): Promise<void> => {
	return invoke<void>('dismiss_applet', { applet });
};

export const getWindows = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('get_windows', args);
};

export const toggleWindow = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_window', args);
};

export const toggleControlCenter = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_control_center', args);
};

/**
 * Closes the control centre outright.
 *
 * The page must not close itself with the toggle: the webview was reparented
 * into a layer surface, so `getCurrentWindow().hide()` would hide the empty
 * toplevel instead, and the toggle can reopen what it was asked to close.
 */
export const hideControlCenter = <T = any>(): Promise<T> => {
	return invoke<T>('hide_control_center');
};

/** Launches the separate vasak-settings application. */
export const openSettings = <T = any>(): Promise<T> => {
	return invoke<T>('open_settings');
};

export const toggleMenu = <T = any>(args?: any): Promise<T> => {
	return invoke<T>('toggle_menu', args);
};

export const toggleSessionPopup = <T = any>(action: string): Promise<T> => {
	return invoke<T>('toggle_session_popup', { action });
};
