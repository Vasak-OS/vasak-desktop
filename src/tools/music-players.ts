import type { PlayerRef } from '@/interfaces/music';

/**
 * Cuál de los reproductores abiertos es el que se está mostrando.
 *
 * Manda el que dice `musicInfo.player`, que es el que llega con cada
 * `music-playing-update`; la marca `active` de la lista es de cuando se pidió
 * la lista, y puede haber quedado vieja. Si ninguno coincide —la lista llegó
 * antes que el primer aviso—, el marcado, y si no, el primero.
 */
export function activePlayerIndex(players: readonly PlayerRef[], current: string): number {
	if (players.length === 0) return 0;
	const byBus = players.findIndex((entry) => entry.player === current);
	if (byBus !== -1) return byBus;
	const flagged = players.findIndex((entry) => entry.active);
	return flagged === -1 ? 0 : flagged;
}

/**
 * El nombre de cada punto: la aplicación, y lo que suena si se sabe.
 *
 * Dos pestañas de Firefox se llaman igual; el título es lo que las separa.
 */
export function playerLabels(players: readonly PlayerRef[]): string[] {
	return players.map((entry) => {
		const name = entry.identity || entry.player.replace(/^org\.mpris\.MediaPlayer2\./, '');
		return entry.title ? `${name}: ${entry.title}` : name;
	});
}
