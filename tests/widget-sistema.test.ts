import { describe, expect, test } from 'bun:test';
import {
	cpuPercent,
	formatRate,
	memoryPercent,
	type SystemStats,
	visibleTiles,
} from '../src/tools/system-stats';
import { WIDGETS } from '../src/tools/widgets/catalog';

const lectura = (over: Partial<SystemStats> = {}): SystemStats => ({
	cpu_total: 1000,
	cpu_idle: 800,
	mem_total_kib: 16_000_000,
	mem_available_kib: 4_000_000,
	temperature_c: 50,
	...over,
});

describe('el widget de estadísticas', () => {
	test('la CPU es lo no ocioso entre dos lecturas', () => {
		const antes = lectura();
		const despues = lectura({ cpu_total: 2000, cpu_idle: 1300 });
		expect(cpuPercent(antes, despues)).toBe(50);
	});

	test('sin lectura previa o sin tiempo transcurrido, cero', () => {
		expect(cpuPercent(null, lectura())).toBe(0);
		expect(cpuPercent(lectura(), lectura())).toBe(0);
	});

	test('la memoria en uso es total menos disponible', () => {
		expect(memoryPercent(lectura())).toBe(75);
		expect(memoryPercent(lectura({ mem_total_kib: 0 }))).toBe(0);
	});

	test('la red se lee en una unidad y un decimal', () => {
		expect(formatRate(10)).toBe('0 kB/s');
		expect(formatRate(2048)).toBe('2 kB/s');
		expect(formatRate(1.5 * 1024 * 1024)).toBe('1.5 MB/s');
	});

	test('una fila deja CPU y memoria; dos filas, todo', () => {
		expect(visibleTiles(120, 400, true)).toEqual(['cpu', 'memory']);
		expect(visibleTiles(252, 400, true)).toEqual(['cpu', 'memory', 'network', 'temperature']);
	});

	test('sin sensor no hay temperatura y un widget angosto va de a uno', () => {
		expect(visibleTiles(252, 400, false)).toEqual(['cpu', 'memory', 'network']);
		expect(visibleTiles(252, 200, true)).toEqual(['cpu', 'memory']);
		expect(visibleTiles(120, 200, true)).toEqual(['cpu']);
	});

	test('está en el catálogo', () => {
		expect(WIDGETS.system.unique).toBe(true);
		expect(WIDGETS.system.min.h).toBe(1);
	});
});
