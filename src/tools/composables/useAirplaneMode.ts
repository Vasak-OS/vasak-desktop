import { computed, readonly, ref } from 'vue';
import {
	AIRPLANE_MODE_EVENT,
	type AirplaneModeState,
	getAirplaneMode,
	setAirplaneMode,
	unblockRadios,
} from '@/services/airplane-mode.service';
import { useBackendToggle } from '@/tools/composables/useBackendToggle';

/**
 * La lectura en curso, compartida: el mosaico del modo y los de Wi-Fi y
 * Bluetooth se montan juntos al abrir los ajustes, y con esto preguntan una
 * sola vez en lugar de tres.
 */
let inFlight: Promise<AirplaneModeState> | null = null;

function readShared(): Promise<AirplaneModeState> {
	inFlight ??= getAirplaneMode().finally(() => {
		inFlight = null;
	});
	return inFlight;
}

/**
 * El modo avión para un componente (vasak-desktop#180): el mosaico lo alterna,
 * y los de Wi-Fi y Bluetooth miran `wlanBlocked` y `bluetoothBlocked` para
 * verse apagados en cuanto el kernel bloquea la radio, sin esperar a que
 * NetworkManager o BlueZ se enteren.
 *
 * Lee el estado una vez al montar —de la copia en memoria del escritorio— y
 * después lo sigue por el evento, sin sondeos. La tecla de avión del teclado
 * llega por el mismo evento.
 */
export function useAirplaneMode() {
	const last = ref<AirplaneModeState | null>(null);
	const toggle = useBackendToggle<AirplaneModeState>({
		name: 'airplane-mode',
		event: AIRPLANE_MODE_EVENT,
		read: readShared,
		write: setAirplaneMode,
		toState: (state) => {
			last.value = state;
			return state;
		},
	});
	return {
		...toggle,
		hardware: computed(() => Boolean(last.value?.hardware)),
		wlanBlocked: computed(() => Boolean(last.value?.wlanBlocked)),
		bluetoothBlocked: computed(() => Boolean(last.value?.bluetoothBlocked)),
		state: readonly(last),
		unblockRadios,
	};
}
