/**
 * Los dobles de lo que `WindowPanelButton` importa desde `@/…`
 * (vasak-desktop#209): el servicio de ventanas, los ayudantes de la bandeja y
 * el registro. La `TrayIconButton` de la librería no se dobla: es la publicada,
 * y es justo sobre ella que se comprueba que llegue la clase de tamaño mínimo.
 */
export const toggleWindow = async () => {};
export const countLabel = (count: number | undefined): string | undefined =>
	typeof count === 'number' && count > 0 ? String(count) : undefined;
export const progressPercent = (progress: number | undefined): number | undefined =>
	typeof progress === 'number' ? progress : undefined;
export const logError = () => {};
