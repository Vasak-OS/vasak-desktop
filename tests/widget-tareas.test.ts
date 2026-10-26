import { describe, expect, test } from 'bun:test';
import type { CalendarTask, TasksReply } from '../src/services/calendar.service';
import { dueToday, isOverdue, rowsFor } from '../src/tools/tasks-widget';
import { WIDGETS } from '../src/tools/widgets/catalog';

const NOW = new Date(2026, 9, 21, 10, 0);
const tarea = (id: string, due: string | null, over: Partial<CalendarTask> = {}): CalendarTask => ({
	task_id: id,
	title: id,
	due,
	all_day: false,
	done: false,
	priority: null,
	...over,
});
const lista = (tasks: CalendarTask[]): TasksReply => ({ state: 'ready', tasks, truncated: false });

describe('el widget de tareas del día', () => {
	test('entran las de hoy y las atrasadas; no las de mañana ni las sin fecha ni las hechas', () => {
		const reply = lista([
			tarea('hoy', new Date(2026, 9, 21, 18).toISOString()),
			tarea('atrasada', new Date(2026, 9, 18, 9).toISOString()),
			tarea('manana', new Date(2026, 9, 22, 9).toISOString()),
			tarea('sin-fecha', null),
			tarea('hecha', new Date(2026, 9, 21, 9).toISOString(), { done: true }),
		]);
		expect(dueToday(reply, NOW).map((t) => t.task_id)).toEqual(['atrasada', 'hoy']);
	});

	test('la prioridad más alta va primero y sin prioridad al final', () => {
		const hoy = new Date(2026, 9, 21, 12).toISOString();
		const reply = lista([
			tarea('sin', hoy),
			tarea('baja', hoy, { priority: 9 }),
			tarea('alta', hoy, { priority: 1 }),
		]);
		expect(dueToday(reply, NOW).map((t) => t.task_id)).toEqual(['alta', 'baja', 'sin']);
	});

	test('sin respuesta lista no hay tareas', () => {
		expect(dueToday(null, NOW)).toEqual([]);
		expect(dueToday({ state: 'denied' }, NOW)).toEqual([]);
	});

	test('una atrasada se distingue de una de hoy', () => {
		expect(isOverdue(tarea('a', new Date(2026, 9, 20, 23).toISOString()), NOW)).toBe(true);
		expect(isOverdue(tarea('b', new Date(2026, 9, 21, 1).toISOString()), NOW)).toBe(false);
		expect(isOverdue(tarea('c', null), NOW)).toBe(false);
	});

	test('las filas siguen al alto de la caja', () => {
		expect(rowsFor(0)).toBe(3);
		expect(rowsFor(120)).toBe(2);
		expect(rowsFor(384)).toBeGreaterThan(rowsFor(120));
		expect(rowsFor(10)).toBe(1);
	});

	test('está en el catálogo', () => {
		expect(WIDGETS.tasks.unique).toBe(true);
	});
});
