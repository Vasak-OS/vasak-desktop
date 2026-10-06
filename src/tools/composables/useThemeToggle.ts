import { setDarkMode, useConfigStore } from '@vasakgroup/plugin-config-manager';
import { computed, onMounted, type Ref, ref } from 'vue';
import { cancelRunningThemeTransitions } from '@/tools/theme.utils';
import { logError } from '@/utils/logger';

/**
 * Pasar del tema claro al oscuro y al revés.
 *
 * Lo usan las dos formas del mismo control en el centro de control: el botón
 * redondo (`ThemeToggle`) y el mosaico del estado B (`ThemeTile`,
 * vasak-desktop#175). Antes la lógica vivía adentro del botón.
 */
export function useThemeToggle() {
	const configStore = ref<any>(null);
	const isSwitching: Ref<boolean> = ref(false);

	const isDark = computed(() => Boolean(configStore.value?.config?.style?.darkmode));

	/** El sol para volver al claro, la luna para pasar al oscuro. */
	const themeIcon = computed(() => (isDark.value ? 'weather-clear' : 'weather-clear-night'));

	onMounted(() => {
		configStore.value = useConfigStore();
	});

	const toggleTheme = async () => {
		if (isSwitching.value || !configStore.value) return;

		isSwitching.value = true;
		try {
			const currentDark = !!configStore.value?.config?.style?.darkmode;
			// Cancelar las transiciones de tema en curso antes de aplicar las nuevas.
			cancelRunningThemeTransitions();
			// Cambiar ya, para que la interfaz responda al instante.
			document.documentElement.classList.toggle('dark', !currentDark);
			await setDarkMode(!currentDark);
		} catch (error) {
			// Volver atrás si falló.
			const currentDark = !!configStore.value?.config?.style?.darkmode;
			document.documentElement.classList.toggle('dark', currentDark);
			logError('Error toggling system theme:', error);
		} finally {
			setTimeout(() => {
				isSwitching.value = false;
			}, 800);
		}
	};

	return { isDark, isSwitching, themeIcon, toggleTheme };
}
