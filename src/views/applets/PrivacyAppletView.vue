<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
import { privacyInUse, privacyStopScreen } from '@/services/core.service';
import { useEventListener } from '@/tools/event.listener';
import { logError } from '@/utils/logger';

/**
 * Quién te está mirando, escuchando o viendo la pantalla.
 *
 * El tooltip del indicador nombra; esto además **corta**, y por eso existe como
 * ventana: un tooltip no puede tener un botón. El diálogo de captura viene
 * prometiendo desde siempre que se puede dejar de compartir «desde el indicador
 * de la barra», y hasta acá esa frase no era cierta.
 *
 * Sólo la pantalla se puede cortar. La cámara y el micrófono los abre la
 * aplicación contra el dispositivo, sin nada en el medio que pueda retirárselo
 * — decirlo con un botón que no funciona sería peor que no tenerlo.
 *
 * Los campos del estado (`camara`, `aplicacion`…) son los que manda el applet de
 * Rust (`applets/privacidad.rs`) y se leen tal cual: renombrarlos es cambiar lo
 * que viaja entre los dos lados, y va junto con ese módulo.
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
const stopping = ref<string | null>(null);

const apply = (state: PrivacyState | null) => {
	camera.value = state?.camara ?? [];
	microphone.value = state?.microfono ?? [];
	screen.value = state?.pantalla ?? [];
};

const load = async () => {
	try {
		apply(await privacyInUse<PrivacyState>());
	} catch (error) {
		logError('[PrivacyApplet] no se pudo consultar el estado:', error);
	}
};

// Mientras la ventana está abierta el estado puede cambiar —una videollamada
// que arranca, una captura que termina— y la lista tiene que seguirlo.
useEventListener<PrivacyState>('privacidad-en-uso', (event) => apply(event.payload));

onMounted(load);

const nobody = computed(
	() => camera.value.length === 0 && microphone.value.length === 0 && screen.value.length === 0
);

const stopScreen = async (session: string) => {
	stopping.value = session;
	try {
		await privacyStopScreen({ session });
		// No se saca de la lista a mano: el agente avisa cuando la sesión se
		// cerró de verdad, y creerle a la interfaz antes que al agente es
		// exactamente cómo se termina diciendo que dejaste de compartir sin que
		// sea cierto.
	} catch (error) {
		logError('[PrivacyApplet] no se pudo cortar la captura:', error);
	} finally {
		stopping.value = null;
	}
};
</script>

<template>
	<AppletPopover applet="privacy" @shown="load">
		<div class="flex h-full min-h-0 flex-col gap-3">
			<header class="min-w-0">
				<h2 class="text-lg font-medium text-tx-main">
					{{ t('views.privacyApplet.title') }}
				</h2>
				<p class="truncate text-xs text-tx-muted">
					{{ t('views.privacyApplet.subtitle') }}
				</p>
			</header>

			<p v-if="nobody" class="text-sm text-tx-muted">
				{{ t('views.privacyApplet.nobody') }}
			</p>

			<div v-else class="flex min-h-0 flex-col gap-4 overflow-y-auto">
				<section v-if="camera.length > 0" class="flex flex-col gap-2">
					<div class="flex items-center gap-2">
						<ThemeIcon name="camera-web" type="symbol" :size="16" />
						<h3 class="text-sm font-medium text-tx-main">
							{{ t('views.privacyApplet.camera') }}
						</h3>
					</div>
					<div
						v-for="usage in camera"
						:key="`camera-${usage.aplicacion}-${usage.detalle}`"
						class="rounded-corner bg-ui-surface/70 px-3 py-2"
					>
						<p class="truncate text-sm text-tx-main">{{ usage.aplicacion }}</p>
						<p class="truncate text-xs text-tx-muted">{{ usage.detalle }}</p>
					</div>
				</section>

				<section v-if="microphone.length > 0" class="flex flex-col gap-2">
					<div class="flex items-center gap-2">
						<ThemeIcon name="microphone-sensitivity-high" type="symbol" :size="16" />
						<h3 class="text-sm font-medium text-tx-main">
							{{ t('views.privacyApplet.microphone') }}
						</h3>
					</div>
					<div
						v-for="usage in microphone"
						:key="`microphone-${usage.aplicacion}-${usage.detalle}`"
						class="rounded-corner bg-ui-surface/70 px-3 py-2"
					>
						<p class="truncate text-sm text-tx-main">{{ usage.aplicacion }}</p>
						<p class="truncate text-xs text-tx-muted">{{ usage.detalle }}</p>
					</div>
				</section>

				<section v-if="screen.length > 0" class="flex flex-col gap-2">
					<div class="flex items-center gap-2">
						<ThemeIcon name="video-display" type="symbol" :size="16" />
						<h3 class="text-sm font-medium text-tx-main">
							{{ t('views.privacyApplet.screen') }}
						</h3>
					</div>
					<div
						v-for="usage in screen"
						:key="`screen-${usage.detalle}`"
						class="flex items-center justify-between gap-2 rounded-corner bg-ui-surface/70 px-3 py-2"
					>
						<p class="min-w-0 truncate text-sm text-tx-main">{{ usage.aplicacion }}</p>
						<button
							type="button"
							class="shrink-0 rounded-corner border border-ui-border px-2 py-1 text-xs font-medium hover:bg-ui-surface disabled:opacity-50"
							:disabled="stopping === usage.detalle"
							@click="stopScreen(usage.detalle)"
						>
							{{
								stopping === usage.detalle
									? t('views.privacyApplet.stopping')
									: t('views.privacyApplet.stop')
							}}
						</button>
					</div>
				</section>
			</div>
		</div>
	</AppletPopover>
</template>
