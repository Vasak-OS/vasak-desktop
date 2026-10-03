/**
 * Lo que va fijo en el panel, al lado del menú (vasak-desktop#151).
 *
 * Hoy son Configuración y Archivos, los dos accesos que el panel tuvo
 * siempre. Es una lista porque ahí van a ir las aplicaciones ancladas: el día
 * que se pueda anclar desde la interfaz, esta lista sale de la configuración
 * en lugar de estar escrita acá.
 *
 * `command` es el nombre con que la capacidad de shell permite lanzarla
 * (`capabilities/default.json`, `shell:allow-spawn`).
 */
export interface PinnedApp {
	id: string;
	/** Nombre del icono en el tema. */
	icon: string;
	/** Clave del catálogo para el nombre accesible y el globo. */
	label: string;
	command: string;
}

export const PINNED_APPS: readonly PinnedApp[] = [
	{
		id: 'settings',
		icon: 'preferences-system',
		label: 'views.panel.settingsAlt',
		command: 'vasak-settings',
	},
	{
		id: 'files',
		icon: 'system-file-manager',
		label: 'views.panel.filesAlt',
		command: 'vasak-file-manager',
	},
];
