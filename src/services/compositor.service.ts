import { invoke } from '@tauri-apps/api/core';

/**
 * Lo que el panel muestra del compositor (vasak-desktop#151): los espacios de
 * trabajo y la distribución de teclado. Los lee Wayfire en el backend
 * (`applets/compositor.rs`), que además avisa con `workspaces-changed` y
 * `keyboard-layout-changed` cuando cambian.
 */

/** Los espacios de la pantalla con foco. */
export interface WorkspaceState {
	/** Cuántos se muestran. */
	count: number;
	/** El actual, desde 0, fila por fila de la grilla de Wayfire. */
	active: number;
	columns: number;
	output_id: number;
}

/** La distribución de teclado. */
export interface KeyboardLayout {
	/** El código corto de la píldora: «US», «LA». */
	short: string;
	/** El nombre entero, para el globo. */
	name: string;
	index: number;
	/** Cuántas hay configuradas: con una sola no hay a qué cambiar. */
	count: number;
}

/** Un rectángulo de la página, en píxeles CSS. */
export interface InputRect {
	x: number;
	y: number;
	width: number;
	height: number;
}

export const getWorkspaces = (): Promise<WorkspaceState | null> =>
	invoke<WorkspaceState | null>('get_workspaces');

export const switchWorkspace = (index: number): Promise<void> =>
	invoke<void>('switch_workspace', { index });

export const getKeyboardLayout = (): Promise<KeyboardLayout | null> =>
	invoke<KeyboardLayout | null>('get_keyboard_layout');

export const nextKeyboardLayout = (): Promise<KeyboardLayout | null> =>
	invoke<KeyboardLayout | null>('next_keyboard_layout');

/** Lo que recibe el puntero en el panel: sólo estos rectángulos. */
export const setPanelInputRegion = (rects: InputRect[]): Promise<void> =>
	invoke<void>('set_panel_input_region', { rects });
