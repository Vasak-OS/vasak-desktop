import type { VolumeInfo } from '@/interfaces/volume';

/**
 * Determines the icon name based on volume level and mute status
 */
export function getVolumeIconName(isMuted: boolean, percentage: number): string {
	if (isMuted) return 'audio-volume-muted-symbolic';
	if (percentage <= 0) return 'audio-volume-muted-symbolic';
	if (percentage <= 33) return 'audio-volume-low-symbolic';
	if (percentage <= 66) return 'audio-volume-medium-symbolic';
	return 'audio-volume-high-symbolic';
}

/**
 * Calculates volume percentage from VolumeInfo
 */
export function calculateVolumePercentage(volumeInfo: VolumeInfo, currentVolume: number): number {
	const range = volumeInfo.max - volumeInfo.min;
	const current = currentVolume - volumeInfo.min;
	return Math.round((current / range) * 100);
}

/**
 * El color del porcentaje del volumen, con tokens del esquema.
 *
 * Silenciado va con el color del error y por encima del 80 % con el del éxito,
 * como antes; lo que cambió es de dónde salen: eran `text-red-500` y
 * `text-green-500`, de la paleta de Tailwind, que no siguen el esquema que
 * eligió la persona.
 */
export function volumePercentageClass(isMuted: boolean, percentage: number): string {
	if (isMuted) return 'text-status-error';
	if (percentage > 80) return 'text-status-success';
	return '';
}

/**
 * El icono del micrófono según su volumen y si está silenciado
 * (vasak-desktop#182): `microphone-sensitivity-*` del tema, el nombre
 * freedesktop que traen Adwaita y Breeze.
 */
export function getMicrophoneIconName(isMuted: boolean, percentage: number): string {
	if (isMuted || percentage <= 0) return 'microphone-sensitivity-muted-symbolic';
	if (percentage <= 33) return 'microphone-sensitivity-low-symbolic';
	if (percentage <= 66) return 'microphone-sensitivity-medium-symbolic';
	return 'microphone-sensitivity-high-symbolic';
}
