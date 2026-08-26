<script lang="ts" setup>
import { invoke } from '@tauri-apps/api/core';
import { writeConfig } from '@vasakgroup/plugin-config-manager';
import { showContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import DesktopClockWidget from '@/components/widgets/DesktopClockWidget.vue';
import FilesWidget from '@/components/widgets/FilesWidget.vue';
import MusicWidget from '@/components/widgets/MusicWidget.vue';
import WeatherWidget from '@/components/widgets/WeatherWidget.vue';
import WidgetHost from '@/components/widgets/WidgetHost.vue';
import {
	CELL_GAP,
	CELL_SIZE,
	defaultLayout,
	firstFreeSlot,
	fitAll,
	GRID_PADDING,
	gridSize,
	overlaps,
	WIDGETS,
	type WidgetPlacement,
	type WidgetType,
} from '@/tools/widgets/catalog';
import { logError } from '@/utils/logger';

/**
 * La capa de widgets del escritorio.
 *
 * Antes los widgets estaban apilados en un flex centrado: no tenían posición ni
 * nada guardado, así que no había nada que mover. Ahora viven en una cuadrícula
 * de celdas fijas, cada uno con su lugar y su tamaño en la configuración.
 */
const props = defineProps<{ config: any }>();

const { t } = useI18n();

const componentes: Record<WidgetType, unknown> = {
	clock: DesktopClockWidget,
	music: MusicWidget,
	weather: WeatherWidget,
	files: FilesWidget,
};

const contenedor = ref<HTMLElement | null>(null);
const ancho = ref(0);
const alto = ref(0);
const editing = ref(false);
const panelAbierto = ref(false);
const placements = ref<WidgetPlacement[]>([]);

const grid = computed(() => gridSize(ancho.value, alto.value));

const disponibles = computed(() =>
	Object.values(WIDGETS).filter((definicion) => {
		if (!componentes[definicion.type]) return false;
		if (!definicion.unique) return true;
		return !placements.value.some((puesto) => puesto.type === definicion.type);
	})
);

/**
 * Lo que ofrece el panel: un widget con variantes aparece una vez por variante.
 *
 * El clima entra en una celda ancha y baja al lado del reloj, o en un cuadro
 * grande con la semana entera. Son dos formas de agregarlo, no una que después
 * hay que descubrir redimensionando.
 */
type OpcionDeWidget = {
	key: string;
	type: WidgetType;
	variant?: string;
	label: string;
	description: string;
	size: { w: number; h: number };
};

const opciones = computed<OpcionDeWidget[]>(() => {
	const lista: OpcionDeWidget[] = [];

	for (const definicion of disponibles.value) {
		const variantes = definicion.variants ?? [];

		if (variantes.length > 1) {
			for (const variante of variantes) {
				lista.push({
					key: `${definicion.type}:${variante.id}`,
					type: definicion.type,
					variant: variante.id,
					label: `${t(definicion.labelKey)} · ${t(variante.labelKey)}`,
					description: t(definicion.descriptionKey),
					size: variante.size,
				});
			}
			continue;
		}

		lista.push({
			key: definicion.type,
			type: definicion.type,
			variant: variantes[0]?.id,
			label: t(definicion.labelKey),
			description: t(definicion.descriptionKey),
			size: definicion.default,
		});
	}

	return lista;
});

/**
 * Lo guardado, o la disposición de siempre para quien nunca configuró nada.
 *
 * Siempre acomodado a la cuadrícula que hay ahora: una disposición guardada en
 * una pantalla más grande traía widgets fuera de borde, y acomodar de a uno los
 * dejaba pisados.
 */
function leerDeConfig(): WidgetPlacement[] {
	const guardados = props.config?.desktop?.widgets;

	const crudos: WidgetPlacement[] =
		Array.isArray(guardados) && guardados.length > 0
			? guardados.filter((w: WidgetPlacement) => w?.type && componentes[w.type] !== undefined)
			: defaultLayout(Boolean(props.config?.desktop?.showfiles));

	return fitAll(crudos, grid.value.columns, grid.value.rows);
}

/**
 * Guarda la disposición.
 *
 * Se escribe la configuración entera porque es como funciona el plugin. Se
 * llama al terminar el arrastre —no en cada celda que se cruza— y al agregar o
 * sacar un widget. Antes sólo se guardaba al salir del modo edición: si la
 * sesión se cortaba antes, el trabajo de acomodar se perdía.
 */
async function guardar() {
	try {
		const actual = props.config ?? {};
		await writeConfig({
			...actual,
			desktop: { ...(actual.desktop ?? {}), widgets: placements.value },
		});
	} catch (error) {
		logError(`No se pudo guardar la disposición de widgets: ${error}`);
	}
}

function medir() {
	const caja = contenedor.value?.getBoundingClientRect();
	if (!caja) return;

	ancho.value = caja.width;
	alto.value = caja.height;

	// Una pantalla más chica que antes puede dejar widgets afuera: se acomodan
	// en vez de quedar invisibles para siempre, y sin quedar uno encima de otro.
	const acomodados = fitAll(placements.value, grid.value.columns, grid.value.rows);

	if (JSON.stringify(acomodados) !== JSON.stringify(placements.value)) {
		placements.value = acomodados;
		void guardar();
	}
}

function mover(id: string, posicion: { x: number; y: number }) {
	const candidato = placements.value.map((puesto) =>
		puesto.id === id ? { ...puesto, ...posicion } : puesto
	);
	// Sin el widget no hay nada que mover: con un id que no está en la
	// cuadrícula, `find` devuelve undefined y seguir leería propiedades de
	// nada. Antes eso iba tapado con un `!`.
	const movido = candidato.find((puesto) => puesto.id === id);
	if (!movido) return;

	// No se permite dejar un widget encima de otro: la cuadrícula pierde sentido
	// si dos cosas ocupan la misma celda.
	if (candidato.some((otro) => otro.id !== id && overlaps(movido, otro))) return;

	placements.value = candidato;
}

function redimensionar(id: string, tamano: { w: number; h: number }) {
	const candidato = placements.value.map((puesto) =>
		puesto.id === id ? { ...puesto, ...tamano } : puesto
	);
	// Sin el widget no hay nada que mover: con un id que no está en la
	// cuadrícula, `find` devuelve undefined y seguir leería propiedades de
	// nada. Antes eso iba tapado con un `!`.
	const cambiado = candidato.find((puesto) => puesto.id === id);
	if (!cambiado) return;

	if (candidato.some((otro) => otro.id !== id && overlaps(cambiado, otro))) return;

	placements.value = candidato;
}

function quitar(id: string) {
	placements.value = placements.value.filter((puesto) => puesto.id !== id);
	void guardar();
}

function agregar(type: WidgetType, variant?: string, tamano?: { w: number; h: number }) {
	const definicion = WIDGETS[type];
	const medida = tamano ?? definicion.default;
	const hueco = firstFreeSlot(placements.value, medida, grid.value.columns, grid.value.rows);

	if (!hueco) {
		logError(`No hay lugar en el escritorio para un widget de ${type}`);
		return;
	}

	placements.value = [
		...placements.value,
		{
			id: `${type}-${Date.now()}`,
			type,
			...hueco,
			...medida,
			variant: variant ?? definicion.variants?.[0]?.id,
		},
	];
	void guardar();
}

function terminarEdicion() {
	editing.value = false;
	panelAbierto.value = false;
	void guardar();
}

/**
 * Se entra a editar con clic derecho en el escritorio, como en Android con un
 * toque largo.
 *
 * La escucha va en la ventana y no en el contenedor de la cuadrícula: fuera del
 * modo edición ese contenedor tiene `pointer-events: none` —para no robarle los
 * clics al escritorio— y entonces el clic derecho no le llegaba nunca. El
 * resultado era que no se podía entrar a editar, y por lo tanto nada se podía
 * mover.
 */
// Si abrir el menú falla —o falla el comando que abre la configuración— hay que
// verlo: una promesa suelta acá termina en un aviso del motor que nadie lee.
const alClicDerecho = (evento: MouseEvent) => {
	abrirEdicion(evento).catch((error) => {
		logError('No se pudo abrir el menú del escritorio:', error);
	});
};

async function abrirEdicion(evento: MouseEvent) {
	// Sólo el clic derecho sobre el fondo. Si viene de un widget o del panel de
	// edición, es asunto de ese componente: el día que los widgets tengan su
	// propio menú contextual, este no se lo puede comer.
	const destino = evento.target as HTMLElement | null;

	if (editing.value || destino?.closest('[data-widget], [data-widget-panel]')) return;

	// Antes el clic derecho entraba directo al modo edición. Eso escondía todo
	// lo demás que uno quiere hacer parado en el escritorio —cambiar el fondo,
	// abrir la configuración— y no había forma de descubrirlo.
	const elegido = await showContextMenu(
		[
			{
				id: 'widgets',
				label: t('widgets.menu.edit'),
				icon: 'preferences-desktop',
			},
			{
				id: 'fondo',
				label: t('widgets.menu.wallpaper'),
				icon: 'preferences-desktop-wallpaper',
			},
			{ type: 'separator' },
			{
				id: 'sistema',
				label: t('widgets.menu.settings'),
				icon: 'preferences-system',
			},
		],
		evento
	);

	switch (elegido?.id) {
		case 'widgets':
			editing.value = true;
			panelAbierto.value = true;
			break;
		case 'fondo':
			await invoke('open_settings_section', { section: 'appearance-wallpaper' });
			break;
		case 'sistema':
			await invoke('open_settings');
			break;
	}
}

let observador: ResizeObserver | null = null;

onMounted(() => {
	placements.value = leerDeConfig();
	medir();
	window.addEventListener('contextmenu', alClicDerecho);

	if (contenedor.value) {
		observador = new ResizeObserver(medir);
		observador.observe(contenedor.value);
	}
});

onUnmounted(() => {
	observador?.disconnect();
	window.removeEventListener('contextmenu', alClicDerecho);
});

// Si la configuración cambia desde otro lado —Ajustes, otro monitor— se relee.
watch(
	() => props.config?.desktop?.widgets,
	() => {
		if (!editing.value) placements.value = leerDeConfig();
	}
);

defineExpose({ abrirEdicion });
</script>

<template>
	<div
		ref="contenedor"
		class="absolute inset-0 z-20"
		:class="editing ? 'pointer-events-auto bg-black/20' : 'pointer-events-none'"
	>
		<div
			class="grid h-full w-full"
			:style="{
				padding: `${GRID_PADDING}px`,
				gap: `${CELL_GAP}px`,
				gridTemplateColumns: `repeat(${grid.columns}, ${CELL_SIZE}px)`,
				gridTemplateRows: `repeat(${grid.rows}, ${CELL_SIZE}px)`,
			}"
		>
			<WidgetHost
				v-for="puesto in placements"
				:key="puesto.id"
				:placement="puesto"
				:editing="editing"
				:columns="grid.columns"
				:rows="grid.rows"
				:min-size="WIDGETS[puesto.type].min"
				:max-size="WIDGETS[puesto.type].max"
				class="pointer-events-auto"
				data-widget
				@move="(posicion) => mover(puesto.id, posicion)"
				@resize="(tamano) => redimensionar(puesto.id, tamano)"
				@commit="guardar()"
				@remove="quitar(puesto.id)"
			>
				<component :is="componentes[puesto.type]" :variant="puesto.variant" />
			</WidgetHost>
		</div>

		<!-- Panel de widgets disponibles, sólo mientras se edita. -->
		<aside
			v-if="editing && panelAbierto"
			data-widget-panel
			class="pointer-events-auto absolute bottom-6 left-1/2 max-h-[40vh] w-[min(90vw,760px)] -translate-x-1/2 overflow-auto rounded-corner border border-ui-border bg-ui-bg/90 p-4 shadow-2xl backdrop-blur-lg"
		>
			<div class="mb-3 flex items-center justify-between">
				<h2 class="text-sm font-semibold uppercase tracking-wide text-tx-muted">
					{{ t('widgets.panelTitle') }}
				</h2>
				<button
					type="button"
					class="rounded-corner bg-primary px-3 py-1 text-sm font-semibold text-tx-on-primary"
					@click="terminarEdicion"
				>
					{{ t('widgets.done') }}
				</button>
			</div>

			<p v-if="opciones.length === 0" class="text-sm text-tx-muted">
				{{ t('widgets.allPlaced') }}
			</p>

			<div v-else class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
				<button
					v-for="opcion in opciones"
					:key="opcion.key"
					type="button"
					class="rounded-corner border border-ui-border bg-ui-surface/40 p-3 text-left transition-colors hover:bg-ui-surface"
					@click="agregar(opcion.type, opcion.variant, opcion.size)"
				>
					<span class="block text-sm font-medium text-tx-main">{{ opcion.label }}</span>
					<span class="block text-xs text-tx-muted">{{ opcion.description }}</span>
				</button>
			</div>
		</aside>
	</div>
</template>
