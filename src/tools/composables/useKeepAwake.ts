import {
	getKeepAwake,
	KEEP_AWAKE_EVENT,
	type KeepAwakeState,
	setKeepAwake,
} from '@/services/keep-awake.service';
import { useBackendToggle } from '@/tools/composables/useBackendToggle';

/**
 * «Mantener despierto» para un componente (vasak-desktop#179). Una lectura al
 * montar y después el evento, sin sondeos: un cambio hecho desde otra ventana
 * llega por el mismo camino.
 */
export function useKeepAwake() {
	return useBackendToggle<KeepAwakeState>({
		name: 'keep-awake',
		event: KEEP_AWAKE_EVENT,
		read: getKeepAwake,
		write: setKeepAwake,
		toState: (state) => state,
	});
}
