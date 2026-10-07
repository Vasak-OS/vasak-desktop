/**
 * Los nombres y los iconos de los perfiles de energía (vasak-desktop#189).
 */

/** Lo que se muestra deshabilitado cuando no hay power-profiles-daemon. */
export const DEFAULT_PROFILES = ['power-saver', 'balanced', 'performance'] as const;

const LABELS: Record<string, string> = {
	'power-saver': 'components.PowerProfileControl.powerSaver',
	balanced: 'components.PowerProfileControl.balanced',
	performance: 'components.PowerProfileControl.performance',
};

/** La clave de traducción de un perfil, o `null` si el demonio trae uno nuevo. */
export function profileLabelKey(profile: string): string | null {
	return LABELS[profile] ?? null;
}

/** El icono del tema para el perfil activo; el equilibrado si no se sabe. */
export function profileIcon(profile: string | null): string {
	if (profile === 'power-saver') return 'power-profile-power-saver-symbolic';
	if (profile === 'performance') return 'power-profile-performance-symbolic';
	return 'power-profile-balanced-symbolic';
}
