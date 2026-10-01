import type { LauncherBadge } from '@/interfaces/tray';

export interface WindowInfo {
	id: string;
	title: string;
	is_minimized: boolean;
	icon: string;
	/** El `app-id` de la ventana: en Wayland, el id de su `.desktop`. */
	app_id?: string;
}

export interface WindowPanelButtonProps {
	id: string;
	title: string;
	is_minimized: boolean;
	icon: string;
	app_id?: string;
	/** Contador y progreso que publica su aplicación, si los hay. */
	launcher?: LauncherBadge;
}
