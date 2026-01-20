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
