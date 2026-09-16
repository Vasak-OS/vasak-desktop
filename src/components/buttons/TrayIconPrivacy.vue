<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import { privacyInUse, togglePrivacyApplet } from '@/services/core.service';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useEventListener } from '@/tools/event.listener';
import { logWarning } from '@/utils/logger';

/**
 * Quién te mira, quién te escucha y quién te ve la pantalla.
 *
 * Es un solo lugar en la barra con un símbolo por dispositivo en uso: uno, dos
 * o tres. No hay un glifo combinado por caso porque con tres dispositivos son
 * siete combinaciones, y siete dibujos distintos a 16 píxeles no se distinguen
 * entre sí — tres símbolos claros uno al lado del otro sí.
 *
 * Se puede hacer clic, y eso es nuevo: antes no, porque no había nada que
 * revocar. Con la captura de pantalla sí lo hay, y el diálogo del portal viene
 * prometiendo desde siempre que se puede dejar de compartir «desde el indicador
 * de la barra».
 *
 * El estado se pregunta al montarse y después se escucha: el panel se destruye
 * y se vuelve a crear al cambiar de monitor, y el escritorio no repite un
 * anuncio igual al anterior.
 */
interface Uso {
	aplicacion: string;
	detalle: string;
}

const { t } = useI18n();

const camara = ref<Uso[]>([]);
const microfono = ref<Uso[]>([]);
const pantalla = ref<Uso[]>([]);

/** Si ya llegó un anuncio, la respuesta de la consulta inicial es vieja. */
const yaLlegoUnAnuncio = ref(false);

useEventListener<{ camara: Uso[]; microfono: Uso[]; pantalla: Uso[] }>(
	'privacidad-en-uso',
	(event) => {
		yaLlegoUnAnuncio.value = true;
		camara.value = event.payload.camara ?? [];
		microfono.value = event.payload.microfono ?? [];
		pantalla.value = event.payload.pantalla ?? [];
	}
);

onMounted(async () => {
	try {
		const estado = await privacyInUse<{ camara: Uso[]; microfono: Uso[]; pantalla: Uso[] }>();
		// La consulta sale antes de que el applet pueda anunciar y vuelve
		// después: aplicarla sin mirar pisaría con la foto vieja lo que acaba
		// de llegar por el evento.
		if (yaLlegoUnAnuncio.value) return;
		camara.value = estado?.camara ?? [];
		microfono.value = estado?.microfono ?? [];
		pantalla.value = estado?.pantalla ?? [];
	} catch (error) {
		// El applet es diferido: si todavía no arrancó, el primer anuncio llega
		// por el evento igual y esto no tiene nada que arreglar.
		logWarning('[TrayIconPrivacy] no se pudo consultar el estado inicial:', error);
	}
});

const iconoCamara = useSymbol('camera-web');
const iconoMicrofono = useSymbol('microphone-sensitivity-high');
const iconoPantalla = useSymbol('video-display');

/** Un símbolo por dispositivo en uso, en orden fijo. */
const simbolos = computed(() => {
	const puestos: { clave: string; icono: string; texto: string }[] = [];
	if (camara.value.length > 0)
		puestos.push({
			clave: 'camara',
			icono: iconoCamara.value,
			texto: t('components.TrayIconPrivacy.camera'),
		});
	if (microfono.value.length > 0)
		puestos.push({
			clave: 'microfono',
			icono: iconoMicrofono.value,
			texto: t('components.TrayIconPrivacy.microphone'),
		});
	if (pantalla.value.length > 0)
		puestos.push({
			clave: 'pantalla',
			icono: iconoPantalla.value,
			texto: t('components.TrayIconPrivacy.screen'),
		});
	return puestos;
});

const visible = computed(() => simbolos.value.length > 0);

/** Una línea por aplicación: pueden ser varias a la vez, y de las tres clases. */
const detalle = computed(() => {
	const lineas = [...camara.value, ...microfono.value, ...pantalla.value].map((uso) =>
		t('components.TrayIconPrivacy.usedBy')
			.replace('{0}', uso.aplicacion)
			.replace('{1}', uso.detalle)
	);
	const titulo = simbolos.value.map((simbolo) => simbolo.texto).join(' · ');
	return [titulo, ...lineas].join('\n');
});

const abrir = async () => {
	try {
		await togglePrivacyApplet();
	} catch (error) {
		logWarning('[TrayIconPrivacy] no se pudo abrir el applet:', error);
	}
};
</script>

<template>
  <div
    v-if="visible"
    class="theme-transition p-1 rounded-corner relative flex items-center gap-1 cursor-pointer hover:bg-primary transition-all duration-300"
    :title="detalle"
    @click="abrir"
  >
    <img
      v-for="simbolo in simbolos"
      :key="simbolo.clave"
      :src="simbolo.icono"
      :alt="simbolo.texto"
      class="h-5.5 w-auto transition-all duration-300"
    />
  </div>
</template>
