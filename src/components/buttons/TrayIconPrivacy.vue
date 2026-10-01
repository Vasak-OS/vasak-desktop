<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon, TrayIconButton } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { privacyInUse } from '@/services/core.service';
import { toggleApplet } from '@/services/window.service';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
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
 *
 * Los campos del estado (`camara`, `aplicacion`…) son los que manda el applet de
 * Rust (`applets/privacidad.rs`) y se leen tal cual.
 */
interface Usage {
	aplicacion: string;
	detalle: string;
}

interface PrivacyState {
	camara?: Usage[];
	microfono?: Usage[];
	pantalla?: Usage[];
}

const { t } = useI18n();

const camera = ref<Usage[]>([]);
const microphone = ref<Usage[]>([]);
const screen = ref<Usage[]>([]);

/** Si ya llegó un anuncio, la respuesta de la consulta inicial es vieja. */
const announced = ref(false);

const apply = (state: PrivacyState | null | undefined) => {
	camera.value = state?.camara ?? [];
	microphone.value = state?.microfono ?? [];
	screen.value = state?.pantalla ?? [];
};

useEventListener<PrivacyState>('privacidad-en-uso', (event) => {
	announced.value = true;
	apply(event.payload);
});

onMounted(async () => {
	try {
		const state = await privacyInUse<PrivacyState>();
		// La consulta sale antes de que el applet pueda anunciar y vuelve
		// después: aplicarla sin mirar pisaría con la foto vieja lo que acaba
		// de llegar por el evento.
		if (announced.value) return;
		apply(state);
	} catch (error) {
		// El applet es diferido: si todavía no arrancó, el primer anuncio llega
		// por el evento igual y esto no tiene nada que arreglar.
		logWarning('[TrayIconPrivacy] no se pudo consultar el estado inicial:', error);
	}
});

/**
 * Un símbolo por dispositivo en uso, en orden fijo.
 *
 * Cada uno tiene el suyo y no hay un glifo combinado: las tres clases dan siete
 * combinaciones, y a dieciséis píxeles no se distinguen entre sí.
 *
 * Lo que se guarda es el **nombre** del icono y no su ruta: lo resuelve
 * `ThemeIcon` en la plantilla, que además lo vuelve a pedir cuando la persona
 * cambia de tema.
 */
const symbols = computed(() => {
	const shown: { key: string; icon: string; text: string }[] = [];
	if (camera.value.length > 0)
		shown.push({
			key: 'camera',
			icon: 'camera-web',
			text: t('components.TrayIconPrivacy.camera'),
		});
	if (microphone.value.length > 0)
		shown.push({
			key: 'microphone',
			icon: 'microphone-sensitivity-high',
			text: t('components.TrayIconPrivacy.microphone'),
		});
	if (screen.value.length > 0)
		shown.push({
			key: 'screen',
			icon: 'video-display',
			text: t('components.TrayIconPrivacy.screen'),
		});
	return shown;
});

const visible = computed(() => symbols.value.length > 0);

/** Una línea por aplicación: pueden ser varias a la vez, y de las tres clases. */
const detail = computed(() => {
	const lines = [...camera.value, ...microphone.value, ...screen.value].map((usage) =>
		t('components.TrayIconPrivacy.usedBy')
			.replace('{0}', usage.aplicacion)
			.replace('{1}', usage.detalle)
	);
	const title = symbols.value.map((symbol) => symbol.text).join(' · ');
	return [title, ...lines].join('\n');
});

/** La instancia del botón de la librería: `toggleApplet` le lee el `$el`. */
const button = ref<{ $el?: Element } | null>(null);
const { openClasses } = useOpenApplet('privacy');

const open = async () => {
	try {
		await toggleApplet('privacy', button.value);
	} catch (error) {
		logWarning('[TrayIconPrivacy] no se pudo abrir el applet:', error);
	}
};
</script>

<template>
  <!-- El botón de la bandeja de la librería, con los símbolos en la ranura en
       vez del icono único: el mismo velo al pasar, el mismo foco y el mismo
       nombre accesible (sale de `alt`) que los otros iconos del panel. -->
  <TrayIconButton
    v-if="visible"
    ref="button"
    :alt="detail"
    :tooltip="detail"
    :custom-class="{ 'flex items-center gap-1': true, ...openClasses }"
    @click="open"
  >
    <ThemeIcon
      v-for="symbol in symbols"
      :key="symbol.key"
      :name="symbol.icon"
      type="symbol"
      :size="22"
      :alt="symbol.text"
    />
  </TrayIconButton>
</template>
