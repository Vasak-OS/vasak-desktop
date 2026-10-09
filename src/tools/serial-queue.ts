/**
 * Una fila que corre las tareas de a una, en el orden en que llegaron.
 *
 * Para pedidos al backend que escriben un mismo estado y tardan distinto: el
 * menú de la bandeja lee el menú de DBus de cada aplicación antes de guardarlo,
 * y dos clics seguidos en dos iconos distintos podían terminar con el menú del
 * primero —el más lento— pisando al del segundo, debajo del icono equivocado.
 *
 * Una tarea que falla no traba a las que vienen detrás: la fila sigue.
 */
export function createSerialQueue() {
	let tail: Promise<unknown> = Promise.resolve();

	return function run<T>(task: () => Promise<T>): Promise<T> {
		const next = tail.catch(() => undefined).then(task);
		tail = next;
		return next;
	};
}
