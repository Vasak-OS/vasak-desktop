<script lang="ts" setup>
import { computed, ref } from 'vue';
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
	(e: 'move', posicion: { x: number; y: number }): void;
	(e: 'resize', tamano: { w: number; h: number }): void;
	(e: 'remove'): void;
}>();

const dragging = ref(false);
const resizing = ref(false);

/** Cuánto mide una celda con su separación: la unidad de todo el arrastre. */
const paso = CELL_SIZE + CELL_GAP;

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
function seguirPuntero(
	inicio: PointerEvent,
	alMover: (celdasX: number, celdasY: number) => void
) {
	const desdeX = inicio.clientX;
	const desdeY = inicio.clientY;

	const mover = (evento: PointerEvent) => {
		alMover(
			Math.round((evento.clientX - desdeX) / paso),
			Math.round((evento.clientY - desdeY) / paso)
		);
	};

	const soltar = () => {
		window.removeEventListener('pointermove', mover);
		window.removeEventListener('pointerup', soltar);
		dragging.value = false;
		resizing.value = false;
	};

	window.addEventListener('pointermove', mover);
	window.addEventListener('pointerup', soltar);
}

function empezarArrastre(evento: PointerEvent) {
	if (!props.editing || evento.button !== 0) return;
	evento.preventDefault();

	const { x, y } = props.placement;
	dragging.value = true;

	seguirPuntero(evento, (celdasX, celdasY) => {
		// Se acota acá y no al guardar: mientras se arrastra, el widget nunca se
		// muestra fuera de la pantalla.
		const nuevoX = Math.min(Math.max(1, x + celdasX), props.columns - props.placement.w + 1);
		const nuevoY = Math.min(Math.max(1, y + celdasY), props.rows - props.placement.h + 1);

		if (nuevoX !== props.placement.x || nuevoY !== props.placement.y) {
			emit('move', { x: nuevoX, y: nuevoY });
		}
	});
}

function empezarRedimensionado(evento: PointerEvent) {
	if (!props.editing || evento.button !== 0) return;
	evento.preventDefault();
	evento.stopPropagation();

	const { w, h } = props.placement;
	resizing.value = true;

	seguirPuntero(evento, (celdasX, celdasY) => {
		const nuevoW = Math.min(
			Math.max(props.minSize.w, w + celdasX),
			Math.min(props.maxSize.w, props.columns - props.placement.x + 1)
		);
		const nuevoH = Math.min(
			Math.max(props.minSize.h, h + celdasY),
			Math.min(props.maxSize.h, props.rows - props.placement.y + 1)
		);

		if (nuevoW !== props.placement.w || nuevoH !== props.placement.h) {
			emit('resize', { w: nuevoW, h: nuevoH });
		}
	});
}
</script>

<template>
	<div
		:style="style"
		class="relative"
		:class="[
			editing ? 'cursor-grab touch-none' : '',
			dragging || resizing ? 'z-50 opacity-90' : '',
		]"
		@pointerdown="empezarArrastre"
	>
		<!--
			El marco de todos los widgets vive acá y no en cada uno: fondo, blur,
			borde y esquinas. Repetirlo en cada componente era lo que hacía que
			cada widget tuviera su propia opacidad y su propio blur —o ninguno—,
			y que agregar uno nuevo empezara con la pregunta de qué clases copiar.

			`container-type: size` también va acá, así lo de adentro puede medirse
			contra su celda sin que cada widget tenga que declararlo.

			El contenido no recibe clics mientras se edita: si los recibiera,
			arrastrar el reproductor de música cambiaría de canción.
		-->
		<div
			style="container-type: size"
			class="h-full w-full overflow-hidden rounded-corner border border-ui-border bg-ui-bg/80 backdrop-blur-md"
			:class="editing ? 'pointer-events-none select-none' : ''"
		>
			<slot />
		</div>

		<template v-if="editing">
			<div
				class="pointer-events-none absolute inset-0 rounded-corner border-2 border-dashed border-primary/70"
			></div>

			<button
				type="button"
				class="absolute -right-2 -top-2 flex h-6 w-6 items-center justify-center rounded-full bg-status-error text-xs font-bold text-white shadow-lg"
				:title="$props.placement.type"
				@pointerdown.stop
				@click.stop="emit('remove')"
			>
				×
			</button>

			<!-- La manija va abajo a la derecha, que es donde la busca todo el mundo. -->
			<div
				class="absolute -bottom-1 -right-1 h-5 w-5 cursor-nwse-resize rounded-tl-corner border-b-2 border-r-2 border-primary bg-ui-bg/80"
				@pointerdown="empezarRedimensionado"
			></div>
		</template>
	</div>
</template>
