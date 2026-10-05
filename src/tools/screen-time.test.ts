import { describe, expect, test } from 'bun:test';
import {
	addDays,
	appsOf,
	categoriesOf,
	dailyAverage,
	dayTotal,
	differenceFromYesterday,
	formatDuration,
	hoursOf,
	isDay,
	isEmpty,
	monthOf,
	monthValues,
	rangeFor,
	type ScreenTimeRange,
	weekOf,
} from './screen-time';

/**
 * Las cuentas del tablero de tiempo de pantalla. Los datos son inventados:
 * ninguna prueba lee el historial de verdad (eso lo dice el backend, que
 * además guarda en un directorio temporal en sus pruebas).
 */

const H = 3_600_000;
const M = 60_000;

const WORDS = {
	hoursMinutes: '{0} h {1} min',
	hours: '{0} h',
	minutes: '{0} min',
	underMinute: 'menos de 1 min',
};

/** La semana del video: del lunes 16 al domingo 22 de marzo. */
const days: ScreenTimeRange['days'] = {
	'2026-03-16': { firefox: 3 * H, kitty: 30 * M },
	'2026-03-17': { firefox: 4 * H },
	'2026-03-18': { telegram: 2 * H },
	'2026-03-19': { firefox: 5 * H },
	'2026-03-20': { firefox: 3 * H + 26 * M },
	'2026-03-21': {
		firefox: H + 49 * M,
		balatro: 40 * M,
		libreoffice: 37 * M,
		telegram: 37 * M,
		kitty: 26 * M + 20_000,
	},
};

describe('las fechas', () => {
	test('la semana va de lunes a domingo, aunque el día sea domingo', () => {
		expect(weekOf('2026-03-18')).toEqual([
			'2026-03-16',
			'2026-03-17',
			'2026-03-18',
			'2026-03-19',
			'2026-03-20',
			'2026-03-21',
			'2026-03-22',
		]);
		expect(weekOf('2026-03-22')[0]).toBe('2026-03-16');
		expect(weekOf('2026-03-16')[0]).toBe('2026-03-16');
	});

	test('sumar días cruza meses, años y el 29 de febrero', () => {
		expect(addDays('2026-03-31', 1)).toBe('2026-04-01');
		expect(addDays('2026-01-01', -1)).toBe('2025-12-31');
		expect(addDays('2028-02-28', 1)).toBe('2028-02-29');
		// En Argentina no hay horario de verano, pero en otras zonas sí: las
		// cuentas son en UTC sobre el texto y no se corren.
		expect(addDays('2026-10-04', 1)).toBe('2026-10-05');
	});

	test('el mes y lo que hay que pedir para dibujar un día', () => {
		expect(monthOf('2026-02-10')).toEqual({
			first: '2026-02-01',
			last: '2026-02-28',
			year: 2026,
			month: 2,
		});
		// El 1 de abril de 2026 es miércoles: la semana empieza en marzo.
		expect(rangeFor('2026-04-01')).toEqual({ from: '2026-03-30', to: '2026-04-30' });
		// El 31 de mayo es domingo; el mes manda en los dos extremos.
		expect(rangeFor('2026-05-31')).toEqual({ from: '2026-05-01', to: '2026-05-31' });
		// Y el día anterior siempre entra, para la diferencia con ayer.
		expect(rangeFor('2026-06-01').from <= '2026-05-31').toBe(true);
	});

	test('un día mal escrito no es un día', () => {
		expect(isDay('2026-03-16')).toBe(true);
		expect(isDay('2026-02-30')).toBe(false);
		expect(isDay('16/03/2026')).toBe(false);
		expect(isDay(null)).toBe(false);
	});
});

describe('el total, el promedio y la diferencia', () => {
	test('el total de un día suma todas las aplicaciones', () => {
		expect(dayTotal(days, '2026-03-16')).toBe(3.5 * H);
		expect(dayTotal(days, '2026-03-22')).toBe(0);
		expect(dayTotal({ x: { a: Number.NaN, b: -5, c: 10 } }, 'x')).toBe(10);
	});

	test('el promedio es de los días de la semana que ya pasaron', () => {
		// El jueves 19: lunes a jueves, (3,5 + 4 + 2 + 5) / 4.
		expect(dailyAverage(days, '2026-03-19', '2026-03-19', '2026-03-01')).toBe((14.5 * H) / 4);
		// Mirando el lunes desde el domingo siguiente: la semana entera, con el
		// domingo sin uso contando como cero.
		const week = 3.5 * H + 4 * H + 2 * H + 5 * H + (3 * H + 26 * M) + dayTotal(days, '2026-03-21');
		expect(dailyAverage(days, '2026-03-16', '2026-03-22', '2026-03-01')).toBe(week / 7);
	});

	test('el promedio no cuenta como cero los días de antes de la historia', () => {
		// El registro se prendió el miércoles: lunes y martes no existen.
		expect(dailyAverage(days, '2026-03-19', '2026-03-19', '2026-03-18')).toBe((7 * H) / 2);
		// Sin historia, no hay promedio.
		expect(dailyAverage(days, '2026-03-19', '2026-03-19', null)).toBeNull();
		// Una semana entera antes del primer día, tampoco.
		expect(dailyAverage(days, '2026-03-09', '2026-03-19', '2026-03-16')).toBeNull();
		// Ni una semana futura.
		expect(dailyAverage(days, '2026-03-30', '2026-03-22', '2026-03-01')).toBeNull();
	});

	test('la diferencia con ayer, para arriba y para abajo', () => {
		expect(differenceFromYesterday(days, '2026-03-17', '2026-03-01')).toBe(30 * M);
		expect(differenceFromYesterday(days, '2026-03-18', '2026-03-01')).toBe(-2 * H);
		expect(differenceFromYesterday(days, '2026-03-22', '2026-03-01')).toBe(
			-dayTotal(days, '2026-03-21')
		);
	});

	test('sin ayer medido no hay diferencia', () => {
		expect(differenceFromYesterday(days, '2026-03-16', '2026-03-16')).toBeNull();
		expect(differenceFromYesterday(days, '2026-03-17', null)).toBeNull();
	});
});

describe('la lista de aplicaciones', () => {
	const apps = {
		firefox: { name: 'Firefox', icon: 'firefox' },
		balatro: { name: 'Balatro', icon: 'balatro' },
		libreoffice: { name: 'LibreOffice', icon: 'libreoffice-startcenter' },
		telegram: { name: 'Telegram', icon: 'telegram' },
	};

	test('de la más usada a la menos, con la parte de la primera', () => {
		const rows = appsOf({ days, apps }, '2026-03-21');

		expect(rows.map((row) => row.name)).toEqual([
			'Firefox',
			'Balatro',
			'LibreOffice',
			'Telegram',
			'kitty',
		]);
		expect(rows[0]?.share).toBe(1);
		expect(rows[1]?.share).toBeCloseTo(40 / 109, 5);
		// Sin datos del backend, el app-id y el icono de reserva.
		expect(rows[4]).toMatchObject({
			appId: 'kitty',
			name: 'kitty',
			icon: 'application-x-executable',
		});
	});

	test('un día sin nada da la lista vacía', () => {
		expect(appsOf({ days, apps }, '2026-03-22')).toEqual([]);
		expect(appsOf({ days: { d: { a: 0 } }, apps: {} }, 'd')).toEqual([]);
	});
});

describe('cómo se escribe una duración', () => {
	test('horas y minutos, sin redondear para arriba', () => {
		expect(formatDuration(4 * H + 13 * M, WORDS)).toBe('4 h 13 min');
		expect(formatDuration(2 * H, WORDS)).toBe('2 h');
		expect(formatDuration(40 * M, WORDS)).toBe('40 min');
		expect(formatDuration(H + 59 * M + 59_000, WORDS)).toBe('1 h 59 min');
	});

	test('menos de un minuto no es cero, y cero es cero', () => {
		expect(formatDuration(30_000, WORDS)).toBe('menos de 1 min');
		expect(formatDuration(0, WORDS)).toBe('0 min');
		expect(formatDuration(-5, WORDS)).toBe('0 min');
		expect(formatDuration(Number.NaN, WORDS)).toBe('0 min');
	});
});

describe('el tablero vacío', () => {
	test('sin días, o con días en cero, está vacío', () => {
		expect(isEmpty({})).toBe(true);
		expect(isEmpty({ '2026-03-16': {} })).toBe(true);
		expect(isEmpty({ '2026-03-16': { firefox: 0 } })).toBe(true);
		expect(isEmpty(days)).toBe(false);
	});

	test('las cuentas de un tablero vacío no rompen ni dicen cosas raras', () => {
		expect(dayTotal({}, '2026-03-16')).toBe(0);
		expect(dailyAverage({}, '2026-03-16', '2026-03-16', null)).toBeNull();
		expect(differenceFromYesterday({}, '2026-03-16', null)).toBeNull();
		expect(appsOf({ days: {}, apps: {} }, '2026-03-16')).toEqual([]);
		expect(monthValues({}, '2026-03-16')).toEqual({});
	});

	test('el mapa del mes tiene sólo los días con uso, por número de día', () => {
		const values = monthValues(days, '2026-03-05');
		expect(Object.keys(values).map(Number)).toEqual([16, 17, 18, 19, 20, 21]);
		expect(values[16]).toBe(3.5 * H);
	});
});

describe('la categoría y las horas, para los informes nuevos', () => {
	const apps = {
		firefox: { name: 'Firefox', icon: 'firefox', category: 'Network' },
		code: { name: 'Code', icon: 'code', category: 'Development' },
		// Sin categoría conocida: cae en ''.
		raro: { name: 'Raro', icon: 'raro' },
	};
	const withDays: Pick<ScreenTimeRange, 'days' | 'apps'> = {
		apps,
		days: { '2026-03-16': { firefox: 3 * H, code: 2 * H, raro: 30 * M } },
	};

	test('appsOf arrastra la categoría de cada aplicación', () => {
		const rows = appsOf(withDays, '2026-03-16');
		expect(rows.find((r) => r.appId === 'firefox')?.category).toBe('Network');
		expect(rows.find((r) => r.appId === 'raro')?.category).toBe('');
	});

	test('categoriesOf suma por categoría, de la más usada a la menos', () => {
		expect(categoriesOf(withDays, '2026-03-16')).toEqual([
			{ category: 'Network', ms: 3 * H },
			{ category: 'Development', ms: 2 * H },
			{ category: '', ms: 30 * M },
		]);
		// Un día sin nada: lista vacía.
		expect(categoriesOf(withDays, '2026-03-17')).toEqual([]);
	});

	test('hoursOf suma el desglose por hora de todas las aplicaciones', () => {
		const hours: ScreenTimeRange['hours'] = {
			'2026-03-16': {
				firefox: Array.from({ length: 24 }, (_, h) => (h === 9 ? 2 * M : 0)),
				code: Array.from({ length: 24 }, (_, h) => (h === 9 ? 1 * M : h === 14 ? 5 * M : 0)),
			},
		};
		const result = hoursOf({ hours }, '2026-03-16');
		expect(result).toHaveLength(24);
		expect(result[9]).toBe(3 * M);
		expect(result[14]).toBe(5 * M);
		expect(result[0]).toBe(0);
	});

	test('un día sin desglose por hora da las 24 en cero', () => {
		const result = hoursOf({ hours: {} }, '2026-03-16');
		expect(result).toHaveLength(24);
		expect(result.every((ms) => ms === 0)).toBe(true);
	});
});
