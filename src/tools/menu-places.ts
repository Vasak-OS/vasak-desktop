/**
 * La columna de lugares del menú de inicio (vasak-desktop#203).
 *
 * Es un conjunto **fijo** —Inicio, Documentos, Descargas, Papelera—, con iconos
 * del tema del sistema, que se abre con el gestor de archivos. Las rutas de las
 * carpetas salen de `user-dirs.dirs` (las mismas que usa el widget de archivos);
 * la papelera es la ubicación virtual `trash:///`, que el gestor sabe abrir.
 *
 * El armado vive acá, aparte de la vista, para poder probarlo sin leer el disco
 * ni montar Vue: recibe el `home` ya resuelto y las carpetas del usuario, y
 * devuelve la lista con sus rutas.
 */

import type { UserDirectory } from '@/interfaces/file';

export interface MenuPlace {
	/** Identidad estable, para la clave de la lista. */
	id: 'home' | 'documents' | 'downloads' | 'trash';
	/** Clave de traducción del nombre. */
	labelKey: string;
	/** Nombre de icono del tema. */
	icon: string;
	/** Ruta o URI que se le pasa al gestor de archivos. */
	path: string;
}

/** La ubicación virtual de la papelera que entiende el gestor de archivos. */
export const TRASH_URI = 'trash:///';

/**
 * Los cuatro lugares, con sus rutas resueltas.
 *
 * `userDirs` son las carpetas de `getUserDirectories`; de ahí salen Documentos y
 * Descargas por su clave XDG. Si falta una, se cae a la ruta de siempre bajo el
 * `home`, para que el lugar siga abriendo algo en vez de desaparecer.
 */
export function buildPlaces(home: string, userDirs: readonly UserDirectory[]): MenuPlace[] {
	const byKey = new Map<string, UserDirectory>();
	for (const dir of userDirs) {
		if (dir.xdgKey) byKey.set(dir.xdgKey, dir);
	}

	const resolve = (key: string, fallback: string): string => byKey.get(key)?.path ?? fallback;

	return [
		{ id: 'home', labelKey: 'views.menu.places.home', icon: 'user-home', path: home },
		{
			id: 'documents',
			labelKey: 'views.menu.places.documents',
			icon: 'folder-documents',
			path: resolve('XDG_DOCUMENTS_DIR', `${home}/Documents`),
		},
		{
			id: 'downloads',
			labelKey: 'views.menu.places.downloads',
			icon: 'folder-download',
			path: resolve('XDG_DOWNLOAD_DIR', `${home}/Downloads`),
		},
		{ id: 'trash', labelKey: 'views.menu.places.trash', icon: 'user-trash', path: TRASH_URI },
	];
}
