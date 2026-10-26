/**
 * Las cuentas del widget de estadísticas del sistema (vasak-desktop#269), sin Vue.
 *
 * El backend devuelve contadores crudos (`system_stats.rs`); lo que se muestra
 * —el uso de CPU entre dos lecturas, el porcentaje de memoria— se calcula acá.
 * La red no se mide de nuevo: es la misma tasa que el applet del panel emite
 * (`network-rate`).
 */

export interface SystemStats {
	cpu_total: number;
	cpu_idle: number;
	mem_total_kib: number;
	mem_available_kib: number;
	temperature_c: number | null;
}

const clampPercent = (value: number): number => Math.min(100, Math.max(0, Math.round(value)));

/** El uso de CPU entre dos lecturas, 0–100. Sin tiempo transcurrido, 0. */
export function cpuPercent(previous: SystemStats | null, next: SystemStats): number {
	if (!previous) return 0;
	const total = next.cpu_total - previous.cpu_total;
	const idle = next.cpu_idle - previous.cpu_idle;
	if (!(total > 0)) return 0;
	return clampPercent(((total - idle) / total) * 100);
}

/** La memoria en uso, 0–100. */
export function memoryPercent(stats: SystemStats): number {
	if (!(stats.mem_total_kib > 0)) return 0;
	return clampPercent(
		((stats.mem_total_kib - stats.mem_available_kib) / stats.mem_total_kib) * 100
	);
}

/** Bytes por segundo en una unidad y un decimal; debajo de 1 kB/s es cero. */
export function formatRate(bytes: number): string {
	if (!(bytes >= 1024)) return '0 kB/s';
	if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} kB/s`;
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB/s`;
}

export type StatTile = 'cpu' | 'memory' | 'network' | 'temperature';

/**
 * Qué mosaicos entran en la caja, por orden de importancia.
 *
 * Una fila de la cuadrícula (120) deja CPU y memoria; con dos filas (252) entran
 * los cuatro. La temperatura se omite si no hay sensor. El ancho también quita:
 * por debajo de 240 sólo entra un mosaico por fila.
 */
export function visibleTiles(height: number, width: number, hasTemperature: boolean): StatTile[] {
	const all: StatTile[] = ['cpu', 'memory', 'network'];
	if (hasTemperature) all.push('temperature');
	const perRow = width > 0 && width < 240 ? 1 : 2;
	const rows = height >= 200 ? 2 : 1;
	return all.slice(0, perRow * rows);
}
