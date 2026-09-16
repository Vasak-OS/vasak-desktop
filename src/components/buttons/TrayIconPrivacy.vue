<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import { privacyInUse } from '@/services/core.service';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useEventListener } from '@/tools/event.listener';
import { logWarning } from '@/utils/logger';

/**
 * Quién te mira y quién te escucha.
 *
 * Aparece sólo cuando hay algo usando la cámara o el micrófono, y se va cuando
 * deja de haberlo. No se puede hacer clic: no hay nada que revocar desde acá,
 * y un resaltado al pasar el mouse prometería un botón que no existe.
 *
 * El estado se pregunta al montarse y después se escucha. Preguntar no es
 * redundante: el panel se destruye y se vuelve a crear cuando cambian los
 * monitores, y el escritorio no repite un anuncio igual al anterior, así que un
 * componente nuevo se quedaría invisible con la cámara encendida hasta el
 * próximo cambio.
 */

interface Uso {
	aplicacion: string;
	detalle: string;
}

const { t } = useI18n();

const camara = ref<Uso[]>([]);
const microfono = ref<Uso[]>([]);

/** Si ya llegó un anuncio, la respuesta de la consulta inicial es vieja. */
const yaLlegoUnAnuncio = ref(false);

useEventListener<{ camara: Uso[]; microfono: Uso[] }>('privacidad-en-uso', (event) => {
	yaLlegoUnAnuncio.value = true;
	camara.value = event.payload.camara ?? [];
	microfono.value = event.payload.microfono ?? [];
});

onMounted(async () => {
	try {
		const estado = await privacyInUse<{ camara: Uso[]; microfono: Uso[] }>();
		// La consulta sale antes de que el applet pueda anunciar y vuelve
		// después: aplicarla sin mirar pisaría con la foto vieja lo que acaba
		// de llegar por el evento.
		if (yaLlegoUnAnuncio.value) return;
		camara.value = estado?.camara ?? [];
		microfono.value = estado?.microfono ?? [];
	} catch (error) {
		// El applet es diferido: si todavía no arrancó, el primer anuncio llega
		// por el evento igual y esto no tiene nada que arreglar.
		logWarning('[TrayIconPrivacy] no se pudo consultar el estado inicial:', error);
	}
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
			.replace('{1}', uso.detalle)
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
