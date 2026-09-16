<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useEventListener } from '@/tools/event.listener';

/**
 * Quién te mira y quién te escucha.
 *
 * Aparece sólo cuando hay algo usando la cámara o el micrófono, y se va cuando
 * deja de haberlo. No se puede hacer clic: no hay nada que revocar desde acá,
 * y un resaltado al pasar el mouse prometería un botón que no existe.
 *
 * El estado inicial llega solo. El applet que lo publica es diferido —arranca
 * después de que el panel pintó— así que este componente ya está escuchando
 * cuando se emite el primer evento.
 */

interface Uso {
	aplicacion: string;
	detalle: string;
}

const { t } = useI18n();

const camara = ref<Uso[]>([]);
const microfono = ref<Uso[]>([]);

useEventListener<{ camara: Uso[]; microfono: Uso[] }>('privacidad-en-uso', (event) => {
	camara.value = event.payload.camara ?? [];
	microfono.value = event.payload.microfono ?? [];
});

const visible = computed(() => camara.value.length > 0 || microfono.value.length > 0);

const simbolo = computed(() => {
	if (camara.value.length > 0 && microfono.value.length > 0) return 'vsk-camera-microphone';
	if (camara.value.length > 0) return 'camera-web';
	return 'microphone-sensitivity-high';
});

const icono = useSymbol(simbolo);

const titulo = computed(() => {
	if (camara.value.length > 0 && microfono.value.length > 0)
		return t('components.TrayIconPrivacy.both');
	if (camara.value.length > 0) return t('components.TrayIconPrivacy.camera');
	return t('components.TrayIconPrivacy.microphone');
});

/** Una línea por aplicación: pueden ser varias a la vez. */
const detalle = computed(() => {
	const lineas = [...camara.value, ...microfono.value].map((uso) =>
		t('components.TrayIconPrivacy.usedBy')
			.replace('{0}', uso.aplicacion)
			.replace('{1}', uso.detalle),
	);
	return [titulo.value, ...lineas].join('\n');
});
</script>

<template>
  <TrayIconButton
    v-if="visible"
    :icon="icono"
    :tooltip="detalle"
    :alt="titulo"
    :interactive="false"
  />
</template>
