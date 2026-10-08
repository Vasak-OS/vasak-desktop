/**
 * Los favoritos del menú de inicio (vasak-desktop#203).
 *
 * Un favorito es una ruta `.desktop` guardada en `menu.favorites`. Se resuelve
 * contra lo que devuelve `getMenuItems()` —la lista `all.apps`—, así que una
 * aplicación que se desinstaló deja de aparecer sin que haya que limpiar nada: la
 * ruta queda guardada pero no resuelve.
 *
 * Fijar y desfijar se hace desde el menú contextual de cada aplicación, sin pasar
 * por el backend. La lógica de la lista vive acá, aparte de las vistas, para
 * poder probarla sin montar Vue.
 */

/** Una aplicación tal como la da `getMenuItems()`. */
export interface MenuApp {
	name: string;
	description: string;
	icon: string;
	path: string;
}

/** Si una ruta está entre los favoritos. */
export function isFavorite(favorites: readonly string[], path: string): boolean {
	return favorites.includes(path);
}

/**
 * Agrega o saca una ruta de la lista, devolviendo una lista nueva.
 *
 * No muta la que recibe: el llamador guarda la que vuelve. Una ruta vacía no se
 * agrega nunca.
 */
export function toggleFavorite(favorites: readonly string[], path: string): string[] {
	if (!path) return [...favorites];
	if (favorites.includes(path)) {
		return favorites.filter((entry) => entry !== path);
	}
	return [...favorites, path];
}

/**
 * Las aplicaciones favoritas, en el orden guardado y sin las que ya no existen.
 *
 * `apps` es la lista plana `all.apps`. Se indexa por `path` una sola vez para no
 * recorrerla por cada favorito.
 */
export function resolveFavorites(
	apps: readonly MenuApp[],
	favorites: readonly string[]
): MenuApp[] {
	const byPath = new Map<string, MenuApp>();
	for (const app of apps) {
		if (app && typeof app.path === 'string') byPath.set(app.path, app);
	}

	const result: MenuApp[] = [];
	for (const path of favorites) {
		const app = byPath.get(path);
		if (app) result.push(app);
	}
	return result;
}

/** La lista plana de todas las aplicaciones a partir del árbol del menú. */
export function allApps(menuData: unknown): MenuApp[] {
	const all = (menuData as Record<string, any> | null)?.all?.apps;
	return Array.isArray(all) ? (all as MenuApp[]) : [];
}
