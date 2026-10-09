<script lang="ts" setup>
import { computed, onUnmounted, ref } from 'vue';
import WidgetFrame from '@/components/widgets/WidgetFrame.vue';
import { CELL_GAP, CELL_SIZE, type WidgetPlacement } from '@/tools/widgets/catalog';

/**
 * El marco de un widget en la cuadrícula.
 *
 * Se ocupa de dos cosas y de ninguna más: ubicarse donde dice su posición, y
 * dejarse mover y redimensionar cuando el escritorio está en modo edición. Lo
 * que va adentro no sabe nada de esto.
 *
 * Fuera del modo edición no escucha ningún evento de puntero: un widget que se
 * moviera de un arrastre accidental sería peor que uno que no se mueve.
 */
const props = defineProps<{
	placement: WidgetPlacement;
	editing: boolean;
	columns: number;
	rows: number;
	minSize: { w: number; h: number };
	maxSize: { w: number; h: number };
}>();

const emit = defineEmits<{
	(e: 'move', position: { x: number; y: number }): void;
	(e: 'resize', size: { w: number; h: number }): void;
	/** El arrastre terminó: es el momento de guardar, no cada celda. */
	(e: 'commit'): void;
	(e: 'remove'): void;
}>();

const dragging = ref(false);
const resizing = ref(false);

/** Cuánto mide una celda con su separación: la unidad de todo el arrastre. */
const step = CELL_SIZE + CELL_GAP;

const style = computed(() => ({
	gridColumn: `${props.placement.x} / span ${props.placement.w}`,
	gridRow: `${props.placement.y} / span ${props.placement.h}`,
}));

/**
 * Convierte el movimiento del puntero en celdas.
 *
 * El imán está acá: se redondea el desplazamiento a celdas enteras, así que el
 * widget salta de celda en celda en vez de quedar a mitad de camino.
 */
function followPointer(start: PointerEvent, onMove: (cellsX: number, cellsY: number) => void) {
	const fromX = start.clientX;
	const fromY = start.clientY;

	const move = (event: PointerEvent) => {
		onMove(Math.round((event.clientX - fromX) / step), Math.round((event.clientY - fromY) / step));
	};

	const release = () => {
		cleanup();
		emit('commit');
	};

	// `pointercancel` importa: si el sistema o el navegador cancelan el gesto
	// —un gesto del touchpad, la ventana que pierde el foco— `pointerup` no
	// llega nunca, y sin esto el widget seguía al puntero para siempre.
	window.addEventListener('pointermove', move);
	window.addEventListener('pointerup', release);
	window.addEventListener('pointercancel', release);

	releaseCurrent = () => {
		window.removeEventListener('pointermove', move);
		window.removeEventListener('pointerup', release);
		window.removeEventListener('pointercancel', release);
		dragging.value = false;
		resizing.value = false;
		releaseCurrent = null;
	};
}

/**
 * Cómo se sueltan las escuchas del arrastre en curso.
 *
 * Se guarda aparte para poder cortarlo también al desmontar: las escuchas
 * viven en la ventana, así que sobrevivían al componente y quedaban moviendo
 * un widget que ya no existe.
 */
let releaseCurrent: (() => void) | null = null;

function cleanup() {
	releaseCurrent?.();
}

onUnmounted(cleanup);

function startDrag(event: PointerEvent) {
	if (!props.editing || event.button !== 0) return;
	event.preventDefault();

	const { x, y } = props.placement;
	dragging.value = true;

	followPointer(event, (cellsX, cellsY) => {
		// Se acota acá y no al guardar: mientras se arrastra, el widget nunca se
		// muestra fuera de la pantalla.
		const newX = Math.min(Math.max(1, x + cellsX), props.columns - props.placement.w + 1);
		const newY = Math.min(Math.max(1, y + cellsY), props.rows - props.placement.h + 1);

		if (newX !== props.placement.x || newY !== props.placement.y) {
			emit('move', { x: newX, y: newY });
		}
	});
}

function startResize(event: PointerEvent) {
	if (!props.editing || event.button !== 0) return;
	event.preventDefault();
	event.stopPropagation();

	const { w, h } = props.placement;
	resizing.value = true;

	followPointer(event, (cellsX, cellsY) => {
		const newW = Math.min(
			Math.max(props.minSize.w, w + cellsX),
			Math.min(props.maxSize.w, props.columns - props.placement.x + 1)
		);
		const newH = Math.min(
			Math.max(props.minSize.h, h + cellsY),
			Math.min(props.maxSize.h, props.rows - props.placement.y + 1)
		);

		if (newW !== props.placement.w || newH !== props.placement.h) {
			emit('resize', { w: newW, h: newH });
		}
	});
}
</script>

<template>
	<!--
	  El marcador va acá y no en cada llamada: todo `WidgetHost` **es** un
	  puesto de widget, y `WidgetLayer` lo busca con `closest('[data-widget]')`
	  para no empezar un arrastre del escritorio encima de uno. Puesto desde
	  afuera dependía de que el atributo cayera al nodo raíz, que es justo lo
	  que `strictTemplates` no deja comprobar.
	-->
	<div
		data-widget
		:style="style"
		class="relative"
		:class="[
			editing ? 'cursor-grab touch-none' : '',
			dragging || resizing ? 'z-50 opacity-90' : '',
		]"
		@pointerdown="startDrag"
	>
		<!-- El marco —la caja, la superficie que flota, la edición— es
		     `WidgetFrame`; acá queda dónde va y cómo se arrastra. -->
		<WidgetFrame :editing="editing" @remove="emit('remove')" @resize-start="startResize">
			<slot />
		</WidgetFrame>
	</div>
</template>
