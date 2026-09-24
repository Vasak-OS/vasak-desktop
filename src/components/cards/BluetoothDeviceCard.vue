<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getDeviceInfo } from '@vasakgroup/plugin-bluetooth-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { DeviceCard } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, type Ref, ref } from 'vue';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const extraInfo: Ref<any> = ref({});
const props = defineProps<{
	device: any;
	actionLabel: string;
	connected?: boolean;
	isConnecting?: boolean;
}>();

// Declarado y no por caída de atributos: sin esto, el `onAction` del padre
// además cae sobre el `DeviceCard` de adentro —que sí declara `action`—, y el
// manejador del padre corre **dos veces** por clic. Comprobado montando el
// patrón con @vue/test-utils: sin declarar, dos; declarándolo, una.
defineEmits<{ action: [] }>();

/**
 * Los **nombres** de los iconos, no sus rutas.
 *
 * Lo que va en `extraInfo` viaja como dato hasta `DeviceCard`, que es quien lo
 * dibuja con `ThemeIcon`. Antes viajaba la ruta ya resuelta, así que este
 * componente tenía que resolver cuatro iconos y volver a pedirlos al cambiar de
 * tema.
 */
const icon = computed(() => props.device.icon || 'bluetooth');

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
		info.push({ icon: 'battery-good-symbolic', text: `${extraInfo.value.battery}%` });
	}
	if (props.device.rssi) {
		info.push({
			icon: 'network-wireless-signal-excellent-symbolic',
			text: `${props.device.rssi} dBm`,
		});
	}
	if (extraInfo.value.manufacturer) {
		info.push({ icon: 'emblem-system-symbolic', text: extraInfo.value.manufacturer });
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
  <!--
    `name` y no `icon`: el de la librería tiene las dos, y `icon` es la ruta ya
    resuelta —está obsoleta y avisa por consola—. Lo que viaja desde acá es el
    **nombre** del icono del tema, que es lo que hace que siga al tema sin que
    este componente tenga que resolver nada.

    `connecting-label` se resuelve acá y no adentro del componente: la librería
    la usan seis aplicaciones y no puede depender de que todas tengan la misma
    clave en su catálogo.
  -->
  <DeviceCard
    :name="icon"
    :title="deviceTitle"
    :subtitle="deviceSubtitle"
    :metadata="deviceMetadata"
    :extra-info="deviceExtraInfo"
    :is-connected="connected"
    :is-connecting="isConnecting"
    :action-label="actionLabel"
    :connecting-label="t('components.DeviceCard.connecting')"
    @action="$emit('action')"
  />
</template>
