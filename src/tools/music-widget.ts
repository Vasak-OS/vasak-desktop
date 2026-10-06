/**
 * Las cuentas del widget de música, sin Vue (vasak-desktop#166).
 *
 * Lo que el widget deriva del estado de MPRIS —si se puede reproducir, qué icono
 * de repetición va, el avance del disco, el volumen en 0–100— vive acá para poder
 * probarse; el componente sólo lo dibuja con las piezas de la librería.
 */
import type { MusicInfo } from '@/interfaces/music';
import { progressRatio } from '@/utils/playback';

/** Reproducir o pausar, según lo que el reproductor diga que acepta ahora. */
export function playPauseAvailable(info: MusicInfo, isPlaying: boolean): boolean {
	if (!info.player) return false;
	return isPlaying ? info.canPause : info.canPlay;
}

/** El icono de repetición: el de «una pista» cuando repite la pista. */
export function loopStatusIconName(
	loopStatus: string | null,
	loopIcon: string,
	loopOneIcon: string
): string {
	return loopStatus === 'Track' ? loopOneIcon : loopIcon;
}

/**
 * Si hay aleatorio, repetición o volumen que mostrar. Cada uno es `null` cuando
 * el reproductor no lo implementa, y entonces no se dibuja.
 */
export function hasExtras(info: MusicInfo): boolean {
	return info.shuffle !== null || info.loopStatus !== null || info.volume !== null;
}

/** El aro de avance del disco, de 0 a 100; sin duración conocida, sin aro. */
export function coverRingProgress(position: number, length: number): number | null {
	return length > 0 ? progressRatio(position, length) * 100 : null;
}

/** El volumen de MPRIS (0–1) en 0–100 para el deslizador. */
export function volumeToPercent(volume: number | null): number {
	return Math.round((volume ?? 0) * 100);
}

/** De vuelta a 0–1, acotado, lo que el deslizador manda en 0–100. */
export function percentToVolume(percent: number): number {
	return Math.min(1, Math.max(0, percent / 100));
}

/** La fracción de la pista para un salto en microsegundos, o `null` sin duración. */
export function seekRatio(micros: number, length: number): number | null {
	return length > 0 ? micros / length : null;
}
