<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import AppletFrame from '@/components/layouts/AppletFrame.vue';
import {
	authorizeTwingateResource,
	getTwingateInfo,
	type TwingateInfo,
	type TwingateResource,
} from '@/services/twingate.service';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

/**
 * Los recursos de Twingate.
 *
 * El indicador del panel decía «Twingate» y nada más: que la VPN estaba
 * levantada, no a qué se podía entrar. Lo que hace falta saber es otra cosa
 * —qué recursos están habilitados, cuánto les falta para vencerse y cuáles hay
 * que autorizar— y eso Twingate lo sabe: se le pregunta a su cliente, que
 * contesta en milisegundos porque habla con el demonio local.
 *
 * Se pide al abrir el applet y no en un temporizador de fondo: los vencimientos
 * se miden en días, no vale despertar la máquina por ellos.
 *
 * Es un applet propio y no una sección del de red: esta lista con sus
 * vencimientos y sus botones es una pantalla en sí misma, y adentro del applet
 * de red empujaba para abajo lo que ese applet tiene que contestar primero.
 */
const { t } = useI18n();

const info = ref<TwingateInfo | null>(null);
const cargando = ref(true);
const autorizando = ref<string | null>(null);

const cargar = async () => {
	try {
		info.value = await getTwingateInfo();
	} catch (error) {
		logError('[twingate] No se pudo leer el estado:', error);
		info.value = null;
	} finally {
		cargando.value = false;
	}
};

onMounted(cargar);

// Esconder el applet no destruye el webview, así que Vue no se monta de nuevo:
// sin esto, la segunda vez que se abre muestra lo que se leyó la primera.
useSharedEvent('window-shown', () => {
	cargando.value = true;
	void cargar();
});

const cerrar = () => {
	try {
		getCurrentWindow().close();
	} catch {
		/* ya estaba cerrada */
	}
};

/** Los que hay que autorizar van primero: es lo único que pide una acción. */
const ordenados = computed<TwingateResource[]>(() => {
	const recursos = [...(info.value?.resources ?? [])];

	return recursos.sort((a, b) => {
		if (a.needs_auth !== b.needs_auth) return a.needs_auth ? -1 : 1;
		return a.name.localeCompare(b.name);
	});
});

const porAutorizar = computed(() => ordenados.value.filter((r) => r.needs_auth));

/** El alias es con lo que se lo llama; si no tiene, la dirección. */
const comoSeLlama = (recurso: TwingateResource) => recurso.alias || recurso.address;

const detalleDeEstado = (recurso: TwingateResource) => {
	if (recurso.needs_auth) return t('components.TwingateArea.needsAuth');
	if (recurso.expires_in) {
		return t('components.TwingateArea.expiresIn').replace('{0}', recurso.expires_in);
	}
	// Un recurso sin autenticación no tiene nada que decir, y uno con un estado
	// que no conocemos se muestra con sus propias palabras.
	return recurso.status || t('components.TwingateArea.noAuthNeeded');
};

const autorizar = async (recurso: TwingateResource) => {
	autorizando.value = recurso.name;

	try {
		await authorizeTwingateResource(recurso.name);
	} catch (error) {
		logError('[twingate] No se pudo pedir la autorización:', error);
	} finally {
		autorizando.value = null;
	}
};
</script>

<template>
	<AppletFrame :close-fn="cerrar">
		<div class="flex h-full min-h-0 flex-col gap-3">
			<header class="flex items-center justify-between gap-2">
				<div class="min-w-0">
					<h2 class="text-lg font-medium text-tx-main">Twingate</h2>
					<p class="truncate text-xs text-tx-muted">
						{{
							info?.connected
								? t('components.TwingateArea.resourceCount').replace(
										'{0}',
										String(ordenados.length)
									)
								: t('components.TwingateArea.disconnected')
						}}
					</p>
				</div>

				<span
					v-if="porAutorizar.length > 0"
					class="shrink-0 rounded-corner bg-status-warning/20 px-2 py-1 text-[11px] font-semibold text-status-warning"
				>
					{{
						t('components.TwingateArea.pendingCount').replace('{0}', String(porAutorizar.length))
					}}
				</span>
			</header>

			<p v-if="cargando" class="text-sm text-tx-muted">
				{{ t('components.TwingateArea.loading') }}
			</p>

			<p v-else-if="!info?.installed" class="text-sm text-tx-muted">
				{{ t('components.TwingateArea.notInstalled') }}
			</p>

			<p v-else-if="ordenados.length === 0" class="text-sm text-tx-muted">
				{{ t('components.TwingateArea.empty') }}
			</p>

			<!-- La lista es lo único que crece: el encabezado queda fijo y acá
			     se desplaza, que con setenta recursos es la diferencia entre
			     poder usarlo y no. -->
			<ul v-else class="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto pr-1">
				<li
					v-for="recurso in ordenados"
					:key="recurso.name"
					class="flex items-center gap-2 rounded-corner border border-ui-border/60 bg-ui-surface/45 px-2 py-1.5"
				>
					<span
						class="h-2 w-2 shrink-0 rounded-full"
						:class="recurso.needs_auth ? 'bg-status-warning' : 'bg-status-success'"
						aria-hidden="true"
					></span>

					<div class="min-w-0 flex-1">
						<p class="truncate text-xs font-medium text-tx-main" :title="recurso.name">
							{{ recurso.name }}
						</p>
						<p class="truncate text-[10px] text-tx-muted" :title="recurso.address">
							{{ comoSeLlama(recurso) }} · {{ detalleDeEstado(recurso) }}
						</p>
					</div>

					<button
						v-if="recurso.needs_auth"
						type="button"
						class="shrink-0 rounded-corner bg-primary px-2 py-1 text-[10px] font-semibold text-tx-on-primary disabled:opacity-50"
						:disabled="autorizando === recurso.name"
						@click="autorizar(recurso)"
					>
						{{
							autorizando === recurso.name
								? t('components.TwingateArea.authorizing')
								: t('components.TwingateArea.authorize')
						}}
					</button>
				</li>
			</ul>
		</div>
	</AppletFrame>
</template>
