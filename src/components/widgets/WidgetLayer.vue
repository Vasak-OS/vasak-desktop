<script lang="ts" setup>
import { writeConfig } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import DesktopClockWidget from '@/components/widgets/DesktopClockWidget.vue';
import MusicWidget from '@/components/widgets/MusicWidget.vue';
import WeatherWidget from '@/components/widgets/WeatherWidget.vue';
import WidgetHost from '@/components/widgets/WidgetHost.vue';
import {
	CELL_GAP,
	CELL_SIZE,
	GRID_PADDING,
	WIDGETS,
	clampToGrid,
	defaultLayout,
	firstFreeSlot,
	gridSize,
	overlaps,
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
	// El de archivos llega en el paso siguiente; hasta entonces no se ofrece.
	files: null,
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

/** Lo guardado, o la disposición de siempre para quien nunca configuró nada. */
function leerDeConfig(): WidgetPlacement[] {
	const guardados = props.config?.desktop?.widgets;

	if (Array.isArray(guardados) && guardados.length > 0) {
		return guardados.filter((w: WidgetPlacement) => w?.type && componentes[w.type] !== undefined);
	}

	return defaultLayout(Boolean(props.config?.desktop?.showfiles));
}

/**
 * Guarda la disposición.
 *
 * Se escribe la configuración entera porque es como funciona el plugin; lo que
 * importa es no hacerlo en cada píxel del arrastre, sino cuando la posición ya
 * cambió de celda.
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
	// en vez de quedar invisibles para siempre.
	const acomodados = placements.value.map((puesto) =>
		clampToGrid(puesto, grid.value.columns, grid.value.rows)
	);

	if (JSON.stringify(acomodados) !== JSON.stringify(placements.value)) {
		placements.value = acomodados;
		void guardar();
	}
}

function mover(id: string, posicion: { x: number; y: number }) {
	const candidato = placements.value.map((puesto) =>
		puesto.id === id ? { ...puesto, ...posicion } : puesto
	);
	const movido = candidato.find((puesto) => puesto.id === id)!;

	// No se permite dejar un widget encima de otro: la cuadrícula pierde sentido
	// si dos cosas ocupan la misma celda.
	if (candidato.some((otro) => otro.id !== id && overlaps(movido, otro))) return;

	placements.value = candidato;
}

function redimensionar(id: string, tamano: { w: number; h: number }) {
	const candidato = placements.value.map((puesto) =>
		puesto.id === id ? { ...puesto, ...tamano } : puesto
	);
	const cambiado = candidato.find((puesto) => puesto.id === id)!;

	if (candidato.some((otro) => otro.id !== id && overlaps(cambiado, otro))) return;

	placements.value = candidato;
}

function quitar(id: string) {
	placements.value = placements.value.filter((puesto) => puesto.id !== id);
	void guardar();
}

function agregar(type: WidgetType) {
	const definicion = WIDGETS[type];
	const hueco = firstFreeSlot(
		placements.value,
		definicion.default,
		grid.value.columns,
		grid.value.rows
	);

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
			...definicion.default,
			variant: definicion.variants?.[0]?.id,
		},
	];
	void guardar();
}

function terminarEdicion() {
	editing.value = false;
	panelAbierto.value = false;
	void guardar();
}

/** Se entra a editar con clic derecho en el escritorio, como en Android con un toque largo. */
function abrirEdicion(evento: MouseEvent) {
	evento.preventDefault();
	editing.value = true;
	panelAbierto.value = true;
}

let observador: ResizeObserver | null = null;

onMounted(() => {
	placements.value = leerDeConfig();
	medir();

	if (contenedor.value) {
		observador = new ResizeObserver(medir);
		observador.observe(contenedor.value);
	}
});

onUnmounted(() => observador?.disconnect());

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
		@contextmenu="abrirEdicion"
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
				@move="(posicion) => mover(puesto.id, posicion)"
				@resize="(tamano) => redimensionar(puesto.id, tamano)"
				@remove="quitar(puesto.id)"
			>
				<component :is="componentes[puesto.type]" :variant="puesto.variant" />
			</WidgetHost>
		</div>

		<!-- Panel de widgets disponibles, sólo mientras se edita. -->
		<aside
			v-if="editing && panelAbierto"
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

			<p v-if="disponibles.length === 0" class="text-sm text-tx-muted">
				{{ t('widgets.allPlaced') }}
			</p>

			<div v-else class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
				<button
					v-for="definicion in disponibles"
					:key="definicion.type"
					type="button"
					class="rounded-corner border border-ui-border bg-ui-surface/40 p-3 text-left transition-colors hover:bg-ui-surface"
					@click="agregar(definicion.type)"
				>
					<span class="block text-sm font-medium text-tx-main">{{ t(definicion.labelKey) }}</span>
					<span class="block text-xs text-tx-muted">{{ t(definicion.descriptionKey) }}</span>
				</button>
			</div>
		</aside>
	</div>
</template>
