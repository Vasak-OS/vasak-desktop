<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ToggleControl, type ToggleIndicator } from '@vasakgroup/vue-libvasak';
import { computed, onMounted } from 'vue';
import { toggleApplet } from '@/services/window.service';
import { useNetworkState } from '@/tools/composables/useNetworkState';

const {
	networkState,
	networkIconName,
	vpnConnected,
	networkAlt,
	getCurrentNetwork,
	refreshVpnStatus,
} = useNetworkState();

const { t } = useI18n();

/**
 * El estado de la conexión, en el punto de la esquina de `ToggleControl`
 * (vue-libvasak 2.2.0): verde y latiendo conectado, el acento por la VPN, el
 * rojo sin red. Antes eran un punto a mano encima del botón **y** un anillo
 * de color en `custom-class` que decía lo mismo; el anillo además era lo único
 * que distinguía la VPN, y un color solo no lo oye nadie: ahora el estado se
 * suma al nombre del botón.
 */
const indicator = computed<ToggleIndicator>(() => {
	if (!networkState.value.is_connected) {
		return { tone: 'error', label: t('components.NetworkControl.disconnected') };
	}
	if (vpnConnected.value) return { tone: 'accent', label: t('components.NetworkControl.vpn') };
	return { tone: 'success', pulse: true, label: t('components.NetworkControl.connected') };
});

/** Cuántas de las cuatro barras van llenas, de 0 a 100 de señal. */
const filledBars = computed(() => Math.ceil(networkState.value.signal_strength / 25));

/** El alto de cada barra, de menor a mayor: 6, 8, 10 y 12 px. */
const BAR_HEIGHT = ['h-1.5', 'h-2', 'h-2.5', 'h-3'] as const;

onMounted(async () => {
	await getCurrentNetwork();
	await refreshVpnStatus();
});
</script>

<template>
	<!-- Las barras de señal van en la ranura `overlay` de la librería, abajo a
	     la izquierda como estaban; el punto de estado es su `indicator`. -->
	<ToggleControl
		class="theme-transition"
		:name="networkIconName"
		:label="networkAlt"
		:is-active="networkState.is_connected"
		:indicator="indicator"
		@click="toggleApplet('network')"
	>
		<template v-if="networkState.is_connected" #overlay>
			<span class="absolute bottom-1 left-1 flex items-end gap-0.5" data-signal-bars>
				<span
					v-for="i in 4"
					:key="i"
					class="w-1 rounded-corner-full bg-primary transition-opacity duration-300"
					:class="[BAR_HEIGHT[i - 1], i <= filledBars ? 'opacity-100' : 'opacity-30']"
				></span>
			</span>
		</template>
	</ToggleControl>
</template>
