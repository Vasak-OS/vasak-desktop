<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getDeviceInfo } from '@vasakgroup/plugin-bluetooth-manager';
import { computed, onMounted, type Ref, ref } from 'vue';
import { useIcon, useSymbol } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';
import DeviceCard from './DeviceCard.vue';

const extraInfo: Ref<any> = ref({});
const props = defineProps<{
	device: any;
	actionLabel: string;
	connected?: boolean;
	isConnecting?: boolean;
}>();

const icon = useIcon(computed(() => props.device.icon || 'bluetooth'));
const batteryIcon = useSymbol('battery-good-symbolic');
const signalIcon = useSymbol('network-wireless-signal-excellent-symbolic');
const tagIcon = useSymbol('emblem-system-symbolic');

const deviceTitle = computed(() => props.device.alias || props.device.name || props.device.address);

const deviceSubtitle = computed(() => props.device.address);

const deviceMetadata = computed(() =>
	props.device.icon || props.device.type ? props.device.type : ''
);

interface ExtraInfoItem {
	icon: string;
	text: string;
}

const deviceExtraInfo = computed<ExtraInfoItem[]>(() => {
	const info: ExtraInfoItem[] = [];
	if (extraInfo.value.battery !== undefined) {
		info.push({ icon: batteryIcon.value, text: `${extraInfo.value.battery}%` });
	}
	if (props.device.rssi) {
		info.push({ icon: signalIcon.value, text: `${props.device.rssi} dBm` });
	}
	if (extraInfo.value.manufacturer) {
		info.push({ icon: tagIcon.value, text: extraInfo.value.manufacturer });
	}
	return info;
});

onMounted(async () => {
	if (props.device.path) {
		try {
			extraInfo.value = await getDeviceInfo(props.device.path);
		} catch (e) {
			logError('Error obteniendo info de dispositivo Bluetooth:', e);
			extraInfo.value = {};
		}
	}
});
</script>

<template>
  <DeviceCard
    :icon="icon"
    :title="deviceTitle"
    :subtitle="deviceSubtitle"
    :metadata="deviceMetadata"
    :extra-info="deviceExtraInfo"
    :is-connected="connected"
    :is-connecting="isConnecting"
    :action-label="actionLabel"
    @action="$emit('action')"
  />
</template>
