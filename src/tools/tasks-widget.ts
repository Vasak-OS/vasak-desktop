/**
 * Las cuentas del widget de tareas del día (vasak-desktop#269), sin Vue.
 *
 * «Del día» son las que vencen hoy o ya vencieron: una tarea sin fecha no es de
 * hoy, y una de mañana todavía no. Salen del almacén local de vasak-accounts, el
 * mismo de los eventos (`calendar.service.ts`).
 */
import type { CalendarTask, TasksReply } from '@/services/calendar.service';

/** Cuántas filas entran por alto de caja: una fila de lista mide unos 28 px. */
export const ROW_HEIGHT = 28;

/** Cuántas tareas entran en una caja de ese alto, descontado el encabezado. */
export function rowsFor(height: number): number {
	if (!Number.isFinite(height) || height <= 0) return 3;
	return Math.max(1, Math.floor((height - 40) / ROW_HEIGHT));
}

/**
 * Las tareas de hoy y atrasadas, las más urgentes primero: la prioridad (1 es la
 * más alta; sin prioridad, al final) y después el vencimiento.
 */
export function dueToday(reply: TasksReply | null | undefined, now: Date): CalendarTask[] {
	if (reply?.state !== 'ready') return [];
	const endOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime();
	return reply.tasks
		.filter((task) => !task.done && task.due && new Date(task.due).getTime() < endOfToday)
		.sort(
			(a, b) =>
				(a.priority ?? 10) - (b.priority ?? 10) ||
				new Date(a.due as string).getTime() - new Date(b.due as string).getTime()
		);
}

/** Si la tarea ya venció antes de hoy: se marca distinta. */
export function isOverdue(task: CalendarTask, now: Date): boolean {
	if (!task.due) return false;
	return (
		new Date(task.due).getTime() <
		new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime()
	);
}
