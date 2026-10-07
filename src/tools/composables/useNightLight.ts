import {
	getNightLightState,
	NIGHT_LIGHT_EVENT,
	type NightLightState,
	setNightLightEnabled,
} from '@/services/night-light.service';
import { useBackendToggle } from '@/tools/composables/useBackendToggle';

/**
 * La luz nocturna para un componente: el mosaico y el botón redondo
 * (vasak-desktop#178).
 *
 * Una lectura al montar y después el evento, sin sondeos. Sin `wlsunset`
 * instalado queda no disponible y tocarla no pide nada.
 */
export function useNightLight() {
	return useBackendToggle<NightLightState>({
		name: 'night-light',
		event: NIGHT_LIGHT_EVENT,
		read: getNightLightState,
		write: setNightLightEnabled,
		toState: (state) => state,
	});
}
