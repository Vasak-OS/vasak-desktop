<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getDeviceInfo } from '@vasakgroup/plugin-bluetooth-manager';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, type Ref, ref } from 'vue';
import { logError } from '@/utils/logger';
import DeviceCard from './DeviceCard.vue';

const icon: Ref<string> = ref('');
const extraInfo: Ref<any> = ref({});
const props = defineProps<{
	device: any;
	actionLabel: string;
	connected?: boolean;
}>();

const deviceTitle = computed(() => props.device.alias || props.device.name || props.device.address);

const deviceSubtitle = computed(() => props.device.address);

const deviceMetadata = computed(() =>
	props.device.icon || props.device.type ? props.device.type : ''
);

const deviceExtraInfo = computed(() => {
	const info: string[] = [];
	if (extraInfo.value.battery !== undefined) {
		info.push(`🔋 ${extraInfo.value.battery}%`);
	}
	if (props.device.rssi) {
		info.push(`📶 ${props.device.rssi} dBm`);
	}
	if (extraInfo.value.manufacturer) {
		info.push(`🏷️ ${extraInfo.value.manufacturer}`);
	}
	return info;
});

onMounted(async () => {
	icon.value = await getIconSource(props.device.icon || 'bluetooth');
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
    :action-label="actionLabel"
    @action="$emit('action')"
  />
</template>
