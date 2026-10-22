import { invoke } from '@tauri-apps/api/core';
import type { ClipboardEntry } from '@/tools/clipboard-filter';

/** El historial del portapapeles, de lo más reciente a lo más viejo. */
export const clipboardHistory = (): Promise<ClipboardEntry[]> => {
	return invoke<ClipboardEntry[]>('clipboard_history');
};

/** Vuelve a poner la entrada `id` en el portapapeles. */
export const clipboardPick = (id: number): Promise<void> => {
	return invoke<void>('clipboard_pick', { id });
};

/** Vacía el historial. No toca el portapapeles vivo, sólo lo recordado. */
export const clipboardClear = (): Promise<void> => {
	return invoke<void>('clipboard_clear');
};

/**
 * Esconde la superposición del historial.
 *
 * Como el centro de control: la página no se cierra sola con el conmutador —el
 * webview vive dentro de una superficie de capa y esconder la ventana vacía de
 * Tauri no haría nada—, sino con este comando.
 */
export const hideClipboard = (): Promise<void> => {
	return invoke<void>('hide_clipboard');
};
