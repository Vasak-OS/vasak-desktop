import { showContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { openApp as sysOpenApp } from '@/services/app.service';
import { dismissMenu } from '@/services/window.service';
import { useMenuConfig } from '@/tools/composables/useMenuConfig';
import { isFavorite, type MenuApp } from '@/tools/menu-favorites';
import { logError } from '@/utils/logger';

/**
 * Lanzar una aplicación del menú y fijarla o desfijarla (vasak-desktop#203).
 *
 * Lo comparten la fila de la lista (`AppMenuCard`), el mosaico (`AppTile`) y la
 * grilla de favoritos (`FavoritesArea`): las tres hacían lo mismo copiado —abrir
 * y esconder el menú, y el menú contextual de fijar/desfijar—, que es lo que
 * marcaba la duplicación de Sonar. Acá vive una sola vez.
 */
export function useAppLauncher() {
	const { t } = useI18n();
	const { menu, toggleFavoritePath } = useMenuConfig();

	/** Abre la aplicación y esconde el menú (esconder, no cerrar: ver `dismissMenu`). */
	const launch = async (app: MenuApp) => {
		try {
			await sysOpenApp({ path: app.path } as any);
		} catch (error) {
			logError('Error al abrir aplicación:', error);
		} finally {
			void dismissMenu();
		}
	};

	/**
	 * Abre el menú contextual para fijar o desfijar `app`, anclado al evento del
	 * clic derecho, y aplica lo elegido.
	 */
	const toggleFavoriteFor = async (app: MenuApp, event: MouseEvent) => {
		event.preventDefault();
		const pinned = isFavorite(menu.value.favorites, app.path);
		try {
			const chosen = await showContextMenu(
				[
					{
						id: 'toggle-favorite',
						label: pinned ? t('views.menu.favorites.unpin') : t('views.menu.favorites.pin'),
						icon: pinned ? 'edit-delete' : 'emblem-favorite',
					},
				],
				event
			);
			if (chosen?.id === 'toggle-favorite') await toggleFavoritePath(app.path);
		} catch (error) {
			logError('No se pudo abrir el menú de la aplicación:', error);
		}
	};

	return { launch, toggleFavoriteFor };
}
