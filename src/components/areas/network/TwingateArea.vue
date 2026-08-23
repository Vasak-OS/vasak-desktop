<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import {
	authorizeTwingateResource,
	getTwingateInfo,
	type TwingateInfo,
	type TwingateResource,
} from '@/services/twingate.service';
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
 */
const { t } = useI18n();

const info = ref<TwingateInfo | null>(null);
const cargando = ref(true);
const autorizando = ref<string | null>(null);
const expandido = ref(false);

/** Cuántos se muestran antes de «ver todos»: la lista real tiene setenta. */
const VISIBLES = 4;

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

/** Los que hay que autorizar van primero: es lo único que pide una acción. */
const ordenados = computed<TwingateResource[]>(() => {
	const recursos = [...(info.value?.resources ?? [])];

	return recursos.sort((a, b) => {
		if (a.needs_auth !== b.needs_auth) return a.needs_auth ? -1 : 1;
		return a.name.localeCompare(b.name);
	});
});

const porAutorizar = computed(() => ordenados.value.filter((r) => r.needs_auth));

const visibles = computed(() =>
	expandido.value ? ordenados.value : ordenados.value.slice(0, VISIBLES)
);

const restantes = computed(() => Math.max(0, ordenados.value.length - VISIBLES));

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
	<!-- Sin el cliente instalado no hay sección: es lo que corresponde en una
	     máquina que no usa Twingate. -->
	<section
		v-if="info?.installed"
		class="flex flex-col gap-2 rounded-corner border border-ui-border bg-ui-surface/45 p-3"
	>
		<header class="flex items-center justify-between gap-2">
			<div class="min-w-0">
				<h3 class="font-medium text-tx-main">Twingate</h3>
				<p class="truncate text-xs text-tx-muted">
					{{
						info.connected
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
				class="shrink-0 rounded-corner bg-status-warning/20 px-2 py-0.5 text-[10px] font-semibold text-status-warning"
			>
				{{ t('components.TwingateArea.pendingCount').replace('{0}', String(porAutorizar.length)) }}
			</span>
		</header>

		<p v-if="cargando" class="text-xs text-tx-muted">
			{{ t('components.TwingateArea.loading') }}
		</p>

		<p v-else-if="info.connected && ordenados.length === 0" class="text-xs text-tx-muted">
			{{ t('components.TwingateArea.empty') }}
		</p>

		<ul v-else-if="info.connected" class="flex flex-col gap-1">
			<li
				v-for="recurso in visibles"
				:key="recurso.name"
				class="flex items-center gap-2 rounded-corner px-2 py-1 hover:bg-ui-surface"
			>
				<span
					class="h-1.5 w-1.5 shrink-0 rounded-full"
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

			<!-- Setenta recursos no entran en un applet; los que importan
			     —los que hay que autorizar— ya están arriba. -->
			<li v-if="restantes > 0 && !expandido">
				<button
					type="button"
					class="w-full rounded-corner px-2 py-1 text-[10px] font-medium text-tx-muted hover:bg-ui-surface"
					@click="expandido = true"
				>
					{{ t('components.TwingateArea.showAll').replace('{0}', String(restantes)) }}
				</button>
			</li>
		</ul>
	</section>
</template>
