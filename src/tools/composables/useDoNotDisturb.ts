import {
	DO_NOT_DISTURB_EVENT,
	type DoNotDisturbState,
	getDoNotDisturb,
	setDoNotDisturb,
} from '@/services/do-not-disturb.service';
import { useBackendToggle } from '@/tools/composables/useBackendToggle';

/**
 * «No molestar» para un componente: el mosaico, el botón redondo y el
 * indicador de la bandeja (vasak-desktop#177).
 *
 * Lee el estado una vez al montar —de la copia en memoria del escritorio, sin
 * cruzar el bus— y después lo sigue por el evento, sin sondeos. Un cambio hecho
 * desde otra ventana o por el modo juego llega por el mismo evento.
 */
export function useDoNotDisturb() {
	return useBackendToggle<DoNotDisturbState>({
		name: 'do-not-disturb',
		event: DO_NOT_DISTURB_EVENT,
		read: getDoNotDisturb,
		write: setDoNotDisturb,
		toState: (state) => state,
	});
}
