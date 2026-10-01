<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, Badge, ListRow, StatusDot } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, onUnmounted, ref } from 'vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
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
 *
 * Es un applet propio y no una sección del de red: esta lista con sus
 * vencimientos y sus botones es una pantalla en sí misma, y adentro del applet
 * de red empujaba para abajo lo que ese applet tiene que contestar primero.
 */
const { t } = useI18n();

const info = ref<TwingateInfo | null>(null);
const loading = ref(true);
const authorizing = ref<string | null>(null);

/**
 * Que el puente con Rust haya fallado no es lo mismo que no tener Twingate
 * instalado, y decir lo segundo cuando pasó lo primero manda a buscar un
 * paquete que ya está puesto.
 */
const failed = ref(false);

const load = async () => {
	try {
		info.value = await getTwingateInfo();
		failed.value = false;
	} catch (error) {
		logError('[twingate] No se pudo leer el estado:', error);
		info.value = null;
		failed.value = true;
	} finally {
		loading.value = false;
	}
};

onMounted(load);

// Esconder el applet no destruye el webview, así que Vue no se monta de nuevo:
// sin esto, la segunda vez que se abre muestra lo que se leyó la primera.
const reload = () => {
	loading.value = true;
	void load();
};

/** Los que hay que autorizar van primero: es lo único que pide una acción. */
const sorted = computed<TwingateResource[]>(() => {
	const list = [...(info.value?.resources ?? [])];

	return list.sort((a, b) => {
		if (a.needs_auth !== b.needs_auth) return a.needs_auth ? -1 : 1;
		return a.name.localeCompare(b.name);
	});
});

const pendingAuth = computed(() => sorted.value.filter((r) => r.needs_auth));

/** El alias es con lo que se lo llama; si no tiene, la dirección. */
const displayName = (resource: TwingateResource) => resource.alias || resource.address;

const statusDetail = (resource: TwingateResource) => {
	if (resource.needs_auth) return t('components.TwingateArea.needsAuth');
	if (resource.expires_in) {
		return t('components.TwingateArea.expiresIn').replace('{0}', resource.expires_in);
	}
	// Un recurso sin autenticación no tiene nada que decir, y uno con un estado
	// que no conocemos se muestra con sus propias palabras.
	return resource.status || t('components.TwingateArea.noAuthNeeded');
};

/** Cada cuánto se vuelve a preguntar mientras se espera la autorización. */
const POLL_INTERVAL = 3000;

/** Cuántas veces: pasado ese rato, el navegador quedó a medio camino y lo que
 * corresponde es dejar de insistir en vez de sondear para siempre. */
const MAX_ATTEMPTS = 20;

let alive = true;
onUnmounted(() => {
	alive = false;
});

/**
 * La autorización no termina cuando el comando vuelve.
 *
 * `twingate auth` abre el navegador y el trámite sigue del otro lado, así que lo
 * único que sabemos al volver es que el cliente arrancó. Sin esto el recurso
 * seguía diciendo «hay que autorizarlo» hasta que alguien cerrara y abriera el
 * applet: ahora se vuelve a preguntar hasta que Twingate diga que ya está.
 */
const authorize = async (resource: TwingateResource) => {
	authorizing.value = resource.name;

	try {
		await authorizeTwingateResource(resource.name);
	} catch (error) {
		logError('[twingate] No se pudo pedir la autorización:', error);
		authorizing.value = null;
		return;
	}

	for (let attempt = 0; attempt < MAX_ATTEMPTS; attempt += 1) {
		await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL));

		// La ventana se cierra al perder el foco —el navegador se lo lleva—, así
		// que esto normalmente se corta solo; al volver a abrirla se lee de nuevo.
		if (!alive) return;

		await load();

		const current = info.value?.resources.find((other) => other.name === resource.name);
		if (!current?.needs_auth) break;
	}

	if (alive) authorizing.value = null;
};
</script>

<template>
	<AppletPopover applet="twingate" @shown="reload">
		<div class="flex h-full min-h-0 flex-col gap-3">
			<header class="flex items-center justify-between gap-2">
				<div class="min-w-0">
					<h2 class="text-lg font-medium text-tx-main">Twingate</h2>
					<p class="truncate text-label-xs text-tx-muted">
						{{
							info?.connected
								? t('components.TwingateArea.resourceCount').replace(
										'{0}',
										String(sorted.length)
									)
								: t('components.TwingateArea.disconnected')
						}}
					</p>
				</div>

				<Badge
					v-if="pendingAuth.length > 0"
					tone="warning"
					:label="t('components.TwingateArea.pendingCount').replace('{0}', String(pendingAuth.length))"
				/>
			</header>

			<p v-if="loading" class="text-label-m text-tx-muted">
				{{ t('components.TwingateArea.loading') }}
			</p>

			<p v-else-if="failed" class="text-label-m text-status-error">
				{{ t('components.TwingateArea.failed') }}
			</p>

			<p v-else-if="!info?.installed" class="text-label-m text-tx-muted">
				{{ t('components.TwingateArea.notInstalled') }}
			</p>

			<p v-else-if="sorted.length === 0" class="text-label-m text-tx-muted">
				{{ t('components.TwingateArea.empty') }}
			</p>

			<!-- La lista es lo único que crece: el encabezado queda fijo y acá
			     se desplaza, que con setenta recursos es la diferencia entre
			     poder usarlo y no. -->
			<ul v-else class="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto pr-1">
				<li v-for="resource in sorted" :key="resource.name">
					<ListRow
						:title="resource.name"
						:description="`${displayName(resource)} · ${statusDetail(resource)}`"
						truncate
						class="border border-ui-line bg-ui-surface/70 py-1.5"
					>
						<template #leading>
							<StatusDot :tone="resource.needs_auth ? 'warning' : 'success'" />
						</template>
						<template v-if="resource.needs_auth" #trailing>
							<ActionButton
								:label="
									authorizing === resource.name
										? t('components.TwingateArea.authorizing')
										: t('components.TwingateArea.authorize')
								"
								size="sm"
								:loading="authorizing === resource.name"
								@click="authorize(resource)"
							/>
						</template>
					</ListRow>
				</li>
			</ul>
		</div>
	</AppletPopover>
</template>
