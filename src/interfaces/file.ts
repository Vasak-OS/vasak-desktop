export interface FileEntry {
	name: string;
	path: string;
	isDirectory: boolean;
	isHidden: boolean;
	size?: string;
	/**
	 * El **nombre** del icono en el tema, no una ruta ni un `data:`.
	 *
	 * Guardar acá el icono ya resuelto es lo que hacía que los iconos del
	 * escritorio se quedaran con los del tema anterior: el dato no se entera de
	 * que el tema cambió. Lo dibuja `ThemeIcon`, que sí se entera.
	 */
	icon: string;
	previewUrl?: string;
	mimeType?: string;
	loadError?: boolean;
}

export interface FileIconMapping {
	ext: string[];
	icon: string;
}

export interface UserDirectory {
	name: string;
	icon: string;
	path: string;
	xdgKey?: string;
}
