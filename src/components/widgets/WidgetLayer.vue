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
	firstFreeSlot,
	fitAll,
	GRID_PADDING,
	gridSize,
	overlaps,
	resolveLayout,
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

const widgetComponents: Record<WidgetType, unknown> = {
	clock: DesktopClockWidget,
	music: MusicWidget,
	weather: WeatherWidget,
	files: FilesWidget,
};

const container = ref<HTMLElement | null>(null);
const width = ref(0);
const height = ref(0);
const editing = ref(false);
const panelOpen = ref(false);
const placements = ref<WidgetPlacement[]>([]);
/** Si lo que se ve es la disposición de siempre y no una guardada. */
const fromDefault = ref(false);

/**
 * Si ya se sabe cuánto mide la capa.
 *
 * Antes de medir, `gridSize(0, 0)` da una cuadrícula de 1×1, y acomodar ahí la
 * disposición de siempre dejaba un reloj de una celda en la esquina —la hora no
 * entraba y se cortaba— y descartaba la música por no tener lugar. Eso era lo
 * que veía un escritorio nuevo, porque nada lo volvía a leer después.
 */
const measured = computed(() => width.value > 0 && height.value > 0);

const grid = computed(() => gridSize(width.value, height.value));

const available = computed(() =>
	Object.values(WIDGETS).filter((definition) => {
		if (!widgetComponents[definition.type]) return false;
		if (!definition.unique) return true;
		return !placements.value.some((placement) => placement.type === definition.type);
	})
);

/**
 * Lo que ofrece el panel: un widget con variantes aparece una vez por variante.
 *
 * El clima entra en una celda ancha y baja al lado del reloj, o en un cuadro
 * grande con la semana entera. Son dos formas de agregarlo, no una que después
 * hay que descubrir redimensionando.
 */
type WidgetOption = {
	key: string;
	type: WidgetType;
	variant?: string;
	label: string;
	description: string;
	size: { w: number; h: number };
};

const options = computed<WidgetOption[]>(() => {
	const list: WidgetOption[] = [];

	for (const definition of available.value) {
		const variants = definition.variants ?? [];

		if (variants.length > 1) {
			for (const variantItem of variants) {
				list.push({
					key: `${definition.type}:${variantItem.id}`,
					type: definition.type,
					variant: variantItem.id,
					label: `${t(definition.labelKey)} · ${t(variantItem.labelKey)}`,
					description: t(definition.descriptionKey),
					size: variantItem.size,
				});
			}
			continue;
		}

		list.push({
			key: definition.type,
			type: definition.type,
			variant: variants[0]?.id,
			label: t(definition.labelKey),
			description: t(definition.descriptionKey),
			size: definition.default,
		});
	}

	return list;
});

/**
 * Lo guardado, o la disposición de siempre para quien nunca configuró nada.
 *
 * Siempre acomodado a la cuadrícula que hay ahora: una disposición guardada en
 * una pantalla más grande traía widgets fuera de borde, y acomodar de a uno los
 * dejaba pisados. Una lista guardada vacía es un escritorio libre, no uno sin
 * configurar: ver `resolveLayout`.
 */
function applyConfig() {
	if (!measured.value) return;

	const result = resolveLayout(
		props.config?.desktop?.widgets,
		Boolean(props.config?.desktop?.showfiles),
		grid.value.columns,
		grid.value.rows
	);

	placements.value = result.placements;
	fromDefault.value = result.fromDefault;
}

/**
 * Guarda la disposición.
 *
 * Se escribe la configuración entera porque es como funciona el plugin. Se
 * llama al terminar el arrastre —no en cada celda que se cruza— y al agregar o
 * sacar un widget. Antes sólo se guardaba al salir del modo edición: si la
 * sesión se cortaba antes, el trabajo de acomodar se perdía.
 */
async function save() {
	try {
		const current = props.config ?? {};
		await writeConfig({
			...current,
			desktop: { ...(current.desktop ?? {}), widgets: placements.value },
		});
		// Desde acá la disposición es de la persona: aunque sea la de siempre
		// tal cual, ya no se recalcula al cambiar la pantalla.
		fromDefault.value = false;
	} catch (error) {
		logError(`No se pudo guardar la disposición de widgets: ${error}`);
	}
}

function measure() {
	const box = container.value?.getBoundingClientRect();
	if (!box) return;

	width.value = box.width;
	height.value = box.height;

	// La de siempre no es de nadie: se vuelve a calcular para la pantalla que
	// hay, en vez de acomodarla y guardarla como si alguien la hubiera elegido.
	if (fromDefault.value || placements.value.length === 0) {
		applyConfig();
		return;
	}

	// Una pantalla más chica que antes puede dejar widgets afuera: se acomodan
	// en vez de quedar invisibles para siempre, y sin quedar uno encima de otro.
	const fitted = fitAll(placements.value, grid.value.columns, grid.value.rows);

	if (JSON.stringify(fitted) !== JSON.stringify(placements.value)) {
		placements.value = fitted;
		void save();
	}
}

function move(id: string, position: { x: number; y: number }) {
	const candidate = placements.value.map((placement) =>
		placement.id === id ? { ...placement, ...position } : placement
	);
	// Sin el widget no hay nada que mover: con un id que no está en la
	// cuadrícula, `find` devuelve undefined y seguir leería propiedades de
	// nada. Antes eso iba tapado con un `!`.
	const moved = candidate.find((placement) => placement.id === id);
	if (!moved) return;

	// No se permite dejar un widget encima de otro: la cuadrícula pierde sentido
	// si dos cosas ocupan la misma celda.
	if (candidate.some((other) => other.id !== id && overlaps(moved, other))) return;

	placements.value = candidate;
}

function resize(id: string, size: { w: number; h: number }) {
	const candidate = placements.value.map((placement) =>
		placement.id === id ? { ...placement, ...size } : placement
	);
	// Sin el widget no hay nada que mover: con un id que no está en la
	// cuadrícula, `find` devuelve undefined y seguir leería propiedades de
	// nada. Antes eso iba tapado con un `!`.
	const changed = candidate.find((placement) => placement.id === id);
	if (!changed) return;

	if (candidate.some((other) => other.id !== id && overlaps(changed, other))) return;

	placements.value = candidate;
}

function remove(id: string) {
	placements.value = placements.value.filter((placement) => placement.id !== id);
	void save();
}

function add(type: WidgetType, variant?: string, size?: { w: number; h: number }) {
	const definition = WIDGETS[type];
	const widgetSize = size ?? definition.default;
	const slot = firstFreeSlot(placements.value, widgetSize, grid.value.columns, grid.value.rows);

	if (!slot) {
		logError(`No hay lugar en el escritorio para un widget de ${type}`);
		return;
	}

	placements.value = [
		...placements.value,
		{
			id: `${type}-${Date.now()}`,
			type,
			...slot,
			...widgetSize,
			variant: variant ?? definition.variants?.[0]?.id,
		},
	];
	void save();
}

function finishEditing() {
	editing.value = false;
	panelOpen.value = false;
	void save();
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
const onContextMenu = (event: MouseEvent) => {
	openEditing(event).catch((error) => {
		logError('No se pudo abrir el menú del escritorio:', error);
	});
};

async function openEditing(event: MouseEvent) {
	// Sólo el clic derecho sobre el fondo. Si viene de un widget o del panel de
	// edición, es asunto de ese componente: el día que los widgets tengan su
	// propio menú contextual, este no se lo puede comer.
	const target = event.target as HTMLElement | null;

	if (editing.value || target?.closest('[data-widget], [data-widget-panel]')) return;

	// Antes el clic derecho entraba directo al modo edición. Eso escondía todo
	// lo demás que uno quiere hacer parado en el escritorio —cambiar el fondo,
	// abrir la configuración— y no había forma de descubrirlo.
	const chosen = await showContextMenu(
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
		event
	);

	switch (chosen?.id) {
		case 'widgets':
			editing.value = true;
			panelOpen.value = true;
			break;
		case 'fondo':
			await invoke('open_settings_section', { section: 'appearance-wallpaper' });
			break;
		case 'sistema':
			await invoke('open_settings');
			break;
	}
}

let observer: ResizeObserver | null = null;

onMounted(() => {
	// Primero medir: leer la configuración antes acomodaba todo en una
	// cuadrícula de 1×1. `measure` ya la aplica cuando no hay nada puesto.
	measure();
	window.addEventListener('contextmenu', onContextMenu);

	if (container.value) {
		observer = new ResizeObserver(measure);
		observer.observe(container.value);
	}
});

onUnmounted(() => {
	observer?.disconnect();
	window.removeEventListener('contextmenu', onContextMenu);
});

// Si la configuración cambia desde otro lado —Ajustes, otro monitor— se relee.
// Los archivos también: la configuración llega después de montar, y sin mirar
// `showfiles` la disposición de siempre se quedaba sin ellos.
watch(
	() => [props.config?.desktop?.widgets, props.config?.desktop?.showfiles],
	() => {
		if (!editing.value) applyConfig();
	}
);

defineExpose({ openEditing });
</script>

<template>
	<div
		ref="container"
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
				v-for="placement in placements"
				:key="placement.id"
				:placement="placement"
				:editing="editing"
				:columns="grid.columns"
				:rows="grid.rows"
				:min-size="WIDGETS[placement.type].min"
				:max-size="WIDGETS[placement.type].max"
				class="pointer-events-auto"
				@move="(position) => move(placement.id, position)"
				@resize="(size) => resize(placement.id, size)"
				@commit="save()"
				@remove="remove(placement.id)"
			>
				<component :is="widgetComponents[placement.type]" :variant="placement.variant" />
			</WidgetHost>
		</div>

		<!-- Panel de widgets disponibles, sólo mientras se edita. -->
		<aside
			v-if="editing && panelOpen"
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
					@click="finishEditing"
				>
					{{ t('widgets.done') }}
				</button>
			</div>

			<p v-if="options.length === 0" class="text-sm text-tx-muted">
				{{ t('widgets.allPlaced') }}
			</p>

			<div v-else class="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
				<button
					v-for="option in options"
					:key="option.key"
					type="button"
					class="rounded-corner border border-ui-border bg-ui-surface/40 p-3 text-left transition-colors hover:bg-ui-surface"
					@click="add(option.type, option.variant, option.size)"
				>
					<span class="block text-sm font-medium text-tx-main">{{ option.label }}</span>
					<span class="block text-xs text-tx-muted">{{ option.description }}</span>
				</button>
			</div>
		</aside>
	</div>
</template>
