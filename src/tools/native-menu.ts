/**
 * Apaga el menú del clic derecho que dibuja el motor del navegador.
 *
 * WebKit ofrece «Recargar», «Inspeccionar elemento» y «Abrir enlace en una
 * ventana nueva» sobre un escritorio que no es una página web: ninguna de esas
 * cosas tiene sentido acá, y la que sí funciona —recargar— deja el panel en un
 * estado que nadie pidió.
 *
 * Esto no le saca el clic derecho a nadie: prevenir el comportamiento por
 * defecto no cancela los escuchas de la página, así que el modo edición de los
 * widgets y el menú del panel siguen abriéndose igual.
 */
export function disableNativeContextMenu(): void {
	// En captura y sobre el documento: el evento se ataja antes de llegar a
	// cualquier elemento, incluidos los que todavía no existen.
	document.addEventListener('contextmenu', (event) => event.preventDefault(), {
		capture: true,
	});
}
