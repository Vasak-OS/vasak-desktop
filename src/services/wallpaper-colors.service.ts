/**
 * «Seguir al fondo» (Vasak-OS/vasak-settings#134): cuando cambia el fondo de
 * pantalla, el acento de todo el sistema cambia para combinar con él, al
 * instante y aunque Configuración esté cerrada.
 *
 * Lo corre el escritorio porque es lo único que está siempre abierto. Y dentro
 * del escritorio, **una sola ventana**: la del fondo del monitor principal
 * (`DesktopView` con la etiqueta `desktop`). El panel, los applets y los
 * fondos de los otros monitores también reciben `config-changed`; si todos lo
 * corrieran, escribirían `custom.json` a la vez.
 *
 * Lo que decide y calcula es `followWallpaper` del plugin (2.10.0), el mismo
 * generador que usa Configuración para «Volver a sacar del fondo». Acá sólo se
 * le dan las piezas:
 * - los píxeles, de `wallpaper_pixels` (Rust), que se los pide a
 *   `vasak-settings --wallpaper pixels`: la lectura del fondo es una sola;
 * - el guardado, `saveUserScheme` del plugin, que hace que todas las
 *   aplicaciones abiertas reapliquen el esquema solas.
 */

import { invoke } from '@tauri-apps/api/core';
import {
	type FollowOutcome,
	type FollowWallpaperDeps,
	followWallpaper,
	getSchemeById,
	readConfig,
	saveUserScheme,
	type WallpaperPixels,
} from '@vasakgroup/plugin-config-manager';

/** Los píxeles del fondo, del cuadro de su miniatura. Ver `wallpaper_colors.rs`. */
export function readWallpaperPixels(path: string): Promise<WallpaperPixels> {
	return invoke<WallpaperPixels>('wallpaper_pixels', { path });
}

/** Las piezas de verdad: la configuración, el esquema y el guardado del plugin. */
export const pluginDeps: FollowWallpaperDeps = {
	readConfig,
	loadScheme: async (id) => (await getSchemeById(id))?.scheme ?? null,
	readPixels: readWallpaperPixels,
	save: saveUserScheme,
};

export type WallpaperFollower = {
	/** Lo que se llama en cada `config-changed` y al montar. */
	sync: () => Promise<FollowOutcome>;
};

/**
 * Un seguidor que corre de a una vez.
 *
 * Su propio guardado dispara otro `config-changed`; si llegara mientras el
 * anterior todavía lee el fondo, los dos verían el fondo como nuevo y lo
 * leerían dos veces. En fila, el segundo ya encuentra el fondo anotado y
 * vuelve `unchanged` sin leer nada.
 */
export function createWallpaperFollower(
	deps: FollowWallpaperDeps = pluginDeps,
	onOutcome: (outcome: FollowOutcome) => void = () => {}
): WallpaperFollower {
	let queue: Promise<FollowOutcome> = Promise.resolve('unchanged');
	return {
		sync: () => {
			queue = queue
				.catch(() => 'unchanged' as const)
				.then(() => followWallpaper(deps))
				.then((outcome) => {
					onOutcome(outcome);
					return outcome;
				});
			return queue;
		},
	};
}
