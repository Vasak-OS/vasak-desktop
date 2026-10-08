import { useConfigStore, type VSKConfig, writeConfig } from '@vasakgroup/plugin-config-manager';
import { computed } from 'vue';
import { type MenuConfig, mergeMenuSection, readMenuConfig } from '@/tools/menu-config';
import { toggleFavorite } from '@/tools/menu-favorites';
import { logError } from '@/utils/logger';

/**
 * Qué muestra el menú de inicio y de qué forma (vasak-desktop#203).
 *
 * Es reactivo como el panel: `App.vue` recarga la configuración con cada
 * `config-changed`, así que cambiar la variante o una opción en Configuración
 * reacomoda el menú sin reiniciar nada. La lectura es la tolerante de
 * `menu-config.ts`; acá sólo se la ata al store.
 *
 * Escribir preserva las claves ajenas: `{ ...config.menu, ...nuevo }`, la misma
 * regla que todo el resto del archivo. Nunca se reescribe la sección entera.
 */
export function useMenuConfig() {
	const configStore = useConfigStore();
	const config = computed(() => (configStore as any).config);
	const menu = computed<MenuConfig>(() => readMenuConfig(config.value));

	/**
	 * Mezcla `partial` en la sección `menu` y lo guarda, dejando el resto del
	 * archivo intacto. Parte de la sección cruda —no de la resuelta— para no
	 * escribir de vuelta todos los valores de fábrica como si la persona los
	 * hubiera elegido.
	 */
	const setMenuConfig = async (partial: Partial<MenuConfig>): Promise<void> => {
		try {
			const current = (config.value ?? {}) as VSKConfig;
			await writeConfig({
				...current,
				menu: mergeMenuSection(current, partial),
			} as VSKConfig);
			// El evento `config-changed` que dispara el backend recarga el store;
			// no hace falta tocarlo a mano.
		} catch (error) {
			logError('No se pudo guardar la configuración del menú:', error);
		}
	};

	/** Fija o desfija una ruta `.desktop` de los favoritos, y lo persiste. */
	const toggleFavoritePath = async (path: string): Promise<void> => {
		await setMenuConfig({ favorites: toggleFavorite(menu.value.favorites, path) });
	};

	return { menu, setMenuConfig, toggleFavoritePath };
}
