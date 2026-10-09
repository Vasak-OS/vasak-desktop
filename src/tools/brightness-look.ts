/**
 * Cómo se ve un deslizador de brillo: el icono según el nivel y el color del
 * porcentaje. Lo comparten el del estado A y los de cada monitor del B.
 */

export function brightnessIcon(percent: number): string {
	if (percent > 66) return 'display-brightness-high-symbolic';
	if (percent > 33) return 'display-brightness-medium-symbolic';
	return 'display-brightness-low-symbolic';
}

/** El porcentaje se resalta arriba del 80 % y se apaga debajo del 20 %. */
export function brightnessPercentageClass(percent: number): string {
	if (percent > 80) return 'text-status-warning';
	if (percent < 20) return 'text-tx-muted';
	return '';
}
