/**
 * Lo que el backend reenvía de cada elemento de la bandeja
 * (StatusNotifierItem), su menú (dbusmenu) y lo que la aplicación publica por
 * `com.canonical.Unity.LauncherEntry`.
 *
 * **Un campo ausente no se dibuja.** El backend no manda cadenas vacías,
 * arreglos vacíos ni iconos sin nombre ni mapa de bits: si un campo está, hay
 * algo que mostrar. Ver `src-tauri/src/structs.rs`.
 */

/** Un icono por nombre del tema, por mapa de bits (PNG en base64), o los dos. */
export interface TrayIcon {
	name?: string;
	data?: string;
}

/** El globo del elemento. La descripción llega como texto plano. */
export interface TrayTooltip {
	icon?: TrayIcon;
	title?: string;
	description?: string;
}

/** Progreso (0 a 1), contador (mayor que cero) y urgencia, sólo lo visible. */
export interface LauncherBadge {
	count?: number;
	progress?: number;
	urgent?: boolean;
}

export type TrayStatus = 'Active' | 'Passive' | 'NeedsAttention';

export interface TrayItem {
	id: string;
	service_name: string;
	bus_name?: string | null;
	icon_name?: string;
	icon_data?: string;
	overlay_icon?: TrayIcon;
	/** Sólo se dibuja con `status === 'NeedsAttention'`. */
	attention_icon?: TrayIcon;
	/** Un nombre del tema o una ruta absoluta. */
	attention_movie_name?: string;
	title?: string;
	tooltip?: TrayTooltip;
	status: TrayStatus;
	category: 'ApplicationStatus' | 'Communications' | 'SystemServices' | 'Hardware';
	menu_path?: string | null;
	/** El elemento sólo tiene menú: el clic principal también lo abre. */
	item_is_menu?: boolean;
	launcher?: LauncherBadge;
}

export interface TrayToggle {
	kind: 'checkmark' | 'radio';
	state: 'on' | 'off' | 'indeterminate';
}

export type MenuDisposition = 'informative' | 'warning' | 'alert';

export interface TrayMenu {
	id: number;
	label: string;
	enabled: boolean;
	visible: boolean;
	type: 'standard' | 'separator' | 'submenu';
	/** Casilla u opción de radio. Ausente si la entrada no es tildable. */
	toggle?: TrayToggle;
	icon?: TrayIcon;
	/** Cada elemento es una pulsación: modificadores y, al final, la tecla. */
	shortcut?: string[][];
	/** `normal` no se manda. */
	disposition?: MenuDisposition;
	children?: TrayMenu[] | null;
}

export interface SystrayPopupPayload {
	icon_id: string;
	icon_data?: string | null;
	tooltip?: TrayTooltip | null;
	status?: TrayStatus | null;
	title: string;
	service_name: string;
	items: TrayMenu[];
}

/** Una aplicación con algo visible en `LauncherEntry`, para las ventanas. */
export interface LauncherEntryView extends LauncherBadge {
	desktop_id: string;
}
