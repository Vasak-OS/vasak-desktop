import { GAME_MODE_EVENT, getGameMode, setGameMode } from '@/services/game-mode.service';
import { useBackendToggle } from '@/tools/composables/useBackendToggle';

/**
 * El modo juego para un componente (vasak-desktop#181). Una lectura al montar
 * y después el evento, sin sondeos. Un escritorio que todavía no tenga el
 * comando deja el mosaico no disponible: la lectura falla y nunca se marca
 * disponible.
 */
export function useGameMode() {
	return useBackendToggle<boolean>({
		name: 'game-mode',
		event: GAME_MODE_EVENT,
		read: getGameMode,
		write: setGameMode,
		toState: (enabled) => ({ available: true, enabled }),
	});
}
