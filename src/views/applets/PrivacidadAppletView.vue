<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import AppletFrame from '@/components/layouts/AppletFrame.vue';
import { privacyInUse, privacyStopScreen } from '@/services/core.service';
import { useSharedEvent } from '@/tools/event.bus';
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
 */
interface Uso {
	aplicacion: string;
	detalle: string;
}

const { t } = useI18n();

const camara = ref<Uso[]>([]);
const microfono = ref<Uso[]>([]);
const pantalla = ref<Uso[]>([]);
const cortando = ref<string | null>(null);

const aplicar = (estado: { camara?: Uso[]; microfono?: Uso[]; pantalla?: Uso[] } | null) => {
	camara.value = estado?.camara ?? [];
	microfono.value = estado?.microfono ?? [];
	pantalla.value = estado?.pantalla ?? [];
};

const cargar = async () => {
	try {
		aplicar(await privacyInUse());
	} catch (error) {
		logError('[PrivacidadApplet] no se pudo consultar el estado:', error);
	}
};

// Mientras la ventana está abierta el estado puede cambiar —una videollamada
// que arranca, una captura que termina— y la lista tiene que seguirlo.
useEventListener<{ camara: Uso[]; microfono: Uso[]; pantalla: Uso[] }>(
	'privacidad-en-uso',
	(event) => aplicar(event.payload)
);

// Esconder no destruye el webview, así que al reabrir hay que volver a pedirlo.
useSharedEvent('window-shown', cargar);

onMounted(cargar);

const nadie = computed(
	() => camara.value.length === 0 && microfono.value.length === 0 && pantalla.value.length === 0
);

const cortar = async (sesion: string) => {
	cortando.value = sesion;
	try {
		await privacyStopScreen({ sesion });
		// No se saca de la lista a mano: el agente avisa cuando la sesión se
		// cerró de verdad, y creerle a la interfaz antes que al agente es
		// exactamente cómo se termina diciendo que dejaste de compartir sin que
		// sea cierto.
	} catch (error) {
		logError('[PrivacidadApplet] no se pudo cortar la captura:', error);
	} finally {
		cortando.value = null;
	}
};

const cerrar = () => {
	getCurrentWindow().close();
};
</script>

<template>
	<AppletFrame :close-fn="cerrar">
		<div class="flex h-full min-h-0 flex-col gap-3">
			<header class="min-w-0">
				<h2 class="text-lg font-medium text-tx-main">
					{{ t('views.privacidadApplet.title') }}
				</h2>
				<p class="truncate text-xs text-tx-muted">
					{{ t('views.privacidadApplet.subtitle') }}
				</p>
			</header>

			<p v-if="nadie" class="text-sm text-tx-muted">
				{{ t('views.privacidadApplet.nobody') }}
			</p>

			<div v-else class="flex min-h-0 flex-col gap-4 overflow-y-auto">
				<section v-if="camara.length > 0" class="flex flex-col gap-2">
					<div class="flex items-center gap-2">
						<ThemeIcon name="camera-web" type="symbol" :size="16" />
						<h3 class="text-sm font-medium text-tx-main">
							{{ t('views.privacidadApplet.camera') }}
						</h3>
					</div>
					<div
						v-for="uso in camara"
						:key="`camara-${uso.aplicacion}-${uso.detalle}`"
						class="rounded-corner bg-ui-surface/70 px-3 py-2"
					>
						<p class="truncate text-sm text-tx-main">{{ uso.aplicacion }}</p>
						<p class="truncate text-xs text-tx-muted">{{ uso.detalle }}</p>
					</div>
				</section>

				<section v-if="microfono.length > 0" class="flex flex-col gap-2">
					<div class="flex items-center gap-2">
						<ThemeIcon name="microphone-sensitivity-high" type="symbol" :size="16" />
						<h3 class="text-sm font-medium text-tx-main">
							{{ t('views.privacidadApplet.microphone') }}
						</h3>
					</div>
					<div
						v-for="uso in microfono"
						:key="`microfono-${uso.aplicacion}-${uso.detalle}`"
						class="rounded-corner bg-ui-surface/70 px-3 py-2"
					>
						<p class="truncate text-sm text-tx-main">{{ uso.aplicacion }}</p>
						<p class="truncate text-xs text-tx-muted">{{ uso.detalle }}</p>
					</div>
				</section>

				<section v-if="pantalla.length > 0" class="flex flex-col gap-2">
					<div class="flex items-center gap-2">
						<ThemeIcon name="video-display" type="symbol" :size="16" />
						<h3 class="text-sm font-medium text-tx-main">
							{{ t('views.privacidadApplet.screen') }}
						</h3>
					</div>
					<div
						v-for="uso in pantalla"
						:key="`pantalla-${uso.detalle}`"
						class="flex items-center justify-between gap-2 rounded-corner bg-ui-surface/70 px-3 py-2"
					>
						<p class="min-w-0 truncate text-sm text-tx-main">{{ uso.aplicacion }}</p>
						<button
							type="button"
							class="shrink-0 rounded-corner border border-ui-border px-2 py-1 text-xs font-medium hover:bg-ui-surface disabled:opacity-50"
							:disabled="cortando === uso.detalle"
							@click="cortar(uso.detalle)"
						>
							{{
								cortando === uso.detalle
									? t('views.privacidadApplet.stopping')
									: t('views.privacidadApplet.stop')
							}}
						</button>
					</div>
				</section>
			</div>
		</div>
	</AppletFrame>
</template>
