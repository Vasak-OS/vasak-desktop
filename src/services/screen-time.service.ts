import { invoke } from '@tauri-apps/api/core';
import { readConfig, writeConfig } from '@vasakgroup/plugin-config-manager';
import type { ScreenTimeRange } from '@/tools/screen-time';

/**
 * El tiempo de pantalla que guarda el backend (`src-tauri/src/screen_time/`).
 *
 * Todo se queda en esta computadora: el backend lee y escribe archivos del
 * usuario, y esto sólo se los pide.
 */

/** Los días de `from` a `to` (`AAAA-MM-DD`, los dos incluidos). */
export const getScreenTime = (from: string, to: string): Promise<ScreenTimeRange> =>
	invoke<ScreenTimeRange>('screen_time_range', { from, to });

/** «Borrar historial»: lo guardado y lo que todavía no se guardó. */
export const clearScreenTime = (): Promise<void> => invoke<void>('screen_time_clear');

/**
 * Prende o apaga el registro: `screen_time.enabled` en `vasak.conf`.
 *
 * Por el gestor de configuración, como cualquier ajuste —el backend lo sigue
 * por `config-changed`—, y leyendo antes la configuración entera: se reescribe
 * el archivo completo, y desde la 2.6 el gestor conserva las claves que no
 * conoce, como ésta.
 */
export const setScreenTimeEnabled = async (enabled: boolean): Promise<void> => {
	const config = (await readConfig()) as
		| (Record<string, unknown> & { screen_time?: Record<string, unknown> })
		| null;
	if (!config) throw new Error('no se pudo leer la configuración');
	const section =
		typeof config.screen_time === 'object' && config.screen_time !== null ? config.screen_time : {};
	await writeConfig({ ...config, screen_time: { ...section, enabled } } as never);
};
