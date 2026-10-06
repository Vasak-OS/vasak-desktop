/**
 * Los mosaicos del estado B del centro de control, en orden (vasak-desktop#175).
 *
 * Sumar un ajuste —No molestar, Luz nocturna, Modo avión, Mantener despierto,
 * Juegos (#177–#181)— es sumar una fila acá con su componente y, si tiene
 * detalle, el panel que se abre dentro del bloque. La vista no cambia.
 *
 * Los componentes se cargan bajo demanda (`import()`): ni el código ni las
 * vigilancias de un mosaico existen hasta que el estado B se abre por primera
 * vez.
 */
import { type Component, defineAsyncComponent } from 'vue';

export type TileId = 'network' | 'bluetooth' | 'theme' | 'screen-time' | 'search';

export interface TileSpec {
	id: TileId;
	/** El mosaico. Si tiene detalle, emite `open` al tocarlo. */
	tile: Component;
	/** El panel que se abre dentro del bloque, con «volver». */
	detail?: Component;
	/** Lo que se le pasa al panel de detalle. */
	detailProps?: Record<string, unknown>;
	/** Sólo si el equipo lo tiene: sin adaptador, no hay mosaico de Bluetooth. */
	requires?: 'bluetooth';
}

export const CONTROL_CENTER_TILES: readonly TileSpec[] = [
	{
		id: 'network',
		tile: defineAsyncComponent(() => import('@/components/controls/tiles/NetworkTile.vue')),
		detail: defineAsyncComponent(() => import('@/components/areas/network/NetworkControlArea.vue')),
		// El panel del applet trae su botón de cerrar; acá se vuelve con «volver».
		detailProps: { hideX: true },
	},
	{
		id: 'bluetooth',
		tile: defineAsyncComponent(() => import('@/components/controls/tiles/BluetoothTile.vue')),
		detail: defineAsyncComponent(
			() => import('@/components/areas/bluetooth/BluetoothControlArea.vue')
		),
		requires: 'bluetooth',
	},
	{
		id: 'theme',
		tile: defineAsyncComponent(() => import('@/components/controls/tiles/ThemeTile.vue')),
	},
	{
		id: 'screen-time',
		tile: defineAsyncComponent(() => import('@/components/controls/tiles/ScreenTimeTile.vue')),
	},
	{
		id: 'search',
		tile: defineAsyncComponent(() => import('@/components/controls/tiles/SearchTile.vue')),
	},
];

/** Los mosaicos que van en este equipo. */
export function availableTiles(
	tiles: readonly TileSpec[],
	capabilities: { bluetooth: boolean }
): TileSpec[] {
	return tiles.filter((tile) => tile.requires !== 'bluetooth' || capabilities.bluetooth);
}
