/**
 * Las fichas del bloque de ajustes que no salen de un mosaico
 * (vasak-desktop#182): las abre la flecha › de un control de abajo —el volumen
 * de salida, el micrófono— y se ven dentro del bloque, como lista → ficha con
 * volver, igual que el detalle de un mosaico.
 *
 * Aparte de `control-center-tiles.ts` porque no son mosaicos: no van en la
 * grilla. Se cargan bajo demanda, como los mosaicos.
 */
import { type Component, defineAsyncComponent } from 'vue';

export type SheetId = 'audio-output' | 'audio-input';

export interface SheetSpec {
	id: SheetId;
	/** El panel que se abre dentro del bloque, con «volver». */
	detail: Component;
	/** Lo que se le pasa al panel. */
	detailProps?: Record<string, unknown>;
}

const AudioDeviceSelector = defineAsyncComponent(
	() => import('@/components/controls/AudioDeviceSelector.vue')
);

export const CONTROL_CENTER_SHEETS: readonly SheetSpec[] = [
	{ id: 'audio-output', detail: AudioDeviceSelector, detailProps: { kind: 'output' } },
	{ id: 'audio-input', detail: AudioDeviceSelector, detailProps: { kind: 'input' } },
];

/** La ficha con ese id, si es una de éstas. */
export function findSheet(
	sheets: readonly SheetSpec[],
	id: string | null | undefined
): SheetSpec | null {
	return sheets.find((sheet) => sheet.id === id) ?? null;
}
