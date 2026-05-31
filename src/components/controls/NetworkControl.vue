<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { onMounted } from 'vue';
import { toggleNetworkApplet } from '@/services/window.service';
import { useNetworkState } from '@/tools/composables/useNetworkState';
import ToggleControl from '../forms/ToggleControl.vue';

const {
	networkState,
	networkIconSrc,
	vpnConnected,
	networkAlt,
	getCurrentNetwork,
	refreshVpnStatus,
} = useNetworkState();

onMounted(async () => {
	await getCurrentNetwork();
	await refreshVpnStatus();
});
</script>

<template>
	<div class="relative inline-block">
		<!-- Indicador de estado -->
		<div
			class="absolute top-1 right-1 w-3 h-3 rounded-full transition-all duration-300"
			:class="networkState.is_connected ? 'bg-status-success animate-pulse' : 'bg-status-error'"
		></div>

		<!-- Indicador de intensidad de señal -->
		<div v-if="networkState.is_connected" class="absolute bottom-1 left-1 flex space-x-0.5">
			<div
				v-for="i in 4"
				:key="i"
				class="w-1 bg-primary rounded-full transition-all duration-300"
				:class="{
					'opacity-100': i <= Math.ceil(networkState.signal_strength / 25),
					'opacity-30': i > Math.ceil(networkState.signal_strength / 25),
				}"
				:style="{ height: `${4 + i * 2}px` }"
			></div>
		</div>

		<ToggleControl
			:icon="networkIconSrc"
			:alt="networkAlt"
			:tooltip="networkAlt"
			:is-active="networkState.is_connected"
			:custom-class="{
				'ring-2 ring-status-success': networkState.is_connected && !vpnConnected,
				'ring-2 ring-primary': vpnConnected,
				'ring-2 ring-status-error': !networkState.is_connected,
			}"
			@click="toggleNetworkApplet"
		/>
	</div>
</template>
