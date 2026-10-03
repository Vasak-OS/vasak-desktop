/**
 * El selector rápido de fondos (vasak-desktop#133): de dónde salen los fondos y
 * cómo se aplica uno.
 *
 * # Un solo camino para aplicar el fondo
 *
 * Se escribe **la misma clave que escribe Configuración** en su pantalla de
 * fondos (`WallpaperView.vue` de vasak-settings): `desktop.wallpaper`, una
 * lista con un solo elemento, en `vasak.conf` por el config-manager. El
 * escritorio ya reacciona a esa clave (`DesktopView.vue`), así que aplicar es
 * escribirla y nada más: no hay un segundo aviso que se pueda separar.
 *
 * Antes de escribirla, un video se prepara con la misma preparación de
 * Configuración —la resolución de la pantalla, 30 fps, sin audio—, que el
 * backend le pide a `vasak-settings --wallpaper prepare` (ver
 * `commands/wallpaper_picker.rs`). Una imagen se escribe tal cual.
 */
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { readConfig, type VSKConfig, writeConfig } from '@vasakgroup/plugin-config-manager';
import type { WallpaperItem } from '@vasakgroup/vue-libvasak';

/** Un fondo como lo devuelve el backend. */
export interface WallpaperEntry {
	path: string;
	thumbnail: string | null;
	video: boolean;
}

interface PreparedWallpaper {
	path: string;
	optimized: boolean;
	detail: string;
}

/** El fondo que está puesto ahora, o `null`. */
export function currentWallpaper(
	config: Pick<VSKConfig, 'desktop'> | null | undefined
): string | null {
	const first = config?.desktop?.wallpaper?.[0];
	return typeof first === 'string' && first.trim() !== '' ? first : null;
}

/** El nombre que se lee de un fondo: el archivo, sin la carpeta ni la extensión. */
export function wallpaperLabel(path: string): string {
	const file = path.split('/').pop() ?? path;
	return file.replace(/\.[^.]+$/, '') || file;
}

/**
 * La fila del carrusel. La miniatura va por el protocolo de assets; sin
 * miniatura —Configuración no está, o falló— el carrusel muestra el icono del
 * tema, no el original de 5K.
 */
export function toCarouselItems(
	entries: readonly WallpaperEntry[],
	toUrl = convertFileSrc
): WallpaperItem[] {
	return entries.map((entry) => ({
		id: entry.path,
		label: wallpaperLabel(entry.path),
		thumbnail: entry.thumbnail ? toUrl(entry.thumbnail) : null,
		video: entry.video,
	}));
}

export async function loadWallpaperCatalog(current: string | null): Promise<WallpaperEntry[]> {
	return invoke<WallpaperEntry[]>('wallpaper_catalog', { current });
}

/**
 * La configuración con el fondo nuevo, sin tocar nada más.
 *
 * Igual que Configuración: `desktop.wallpaper` pasa a ser `[ruta]` y el resto de
 * `desktop` —la pausa con batería, los widgets, el tamaño de los iconos— queda
 * como estaba. Aparte para poder probar exactamente qué se escribe.
 */
export function withWallpaper(config: VSKConfig, path: string): VSKConfig {
	return {
		...config,
		desktop: {
			...config.desktop,
			wallpaper: [path],
		},
	};
}

/**
 * Aplica un fondo: lo prepara si es un video y escribe la clave.
 *
 * Se relee la configuración justo antes de escribirla, no se usa una copia de
 * cuando se abrió el selector: en el medio pudo cambiar otra cosa (el tema, un
 * widget) y escribir la copia vieja la desharía.
 */
export async function applyWallpaper(path: string): Promise<string> {
	const prepared = await invoke<PreparedWallpaper>('prepare_wallpaper', { path });
	const config = await readConfig();
	if (!config) throw new Error('no se pudo leer la configuración');
	await writeConfig(withWallpaper(config, prepared.path));
	return prepared.path;
}

export async function hideWallpaperPicker(): Promise<void> {
	await invoke('hide_wallpaper_picker');
}

export async function toggleWallpaperPicker(): Promise<void> {
	await invoke('toggle_wallpaper_picker');
}

export async function openWallpaperSettings(): Promise<void> {
	await invoke('open_settings_section', { section: 'appearance-wallpaper' });
}
