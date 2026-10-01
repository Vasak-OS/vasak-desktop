<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton } from '@vasakgroup/vue-libvasak';
import { computed, onUnmounted, ref } from 'vue';
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

const { t } = useI18n();

const emit = defineEmits<{
	(e: 'move', posicion: { x: number; y: number }): void;
	(e: 'resize', tamano: { w: number; h: number }): void;
	/** El arrastre terminó: es el momento de guardar, no cada celda. */
	(e: 'commit'): void;
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
function seguirPuntero(inicio: PointerEvent, alMover: (celdasX: number, celdasY: number) => void) {
	const desdeX = inicio.clientX;
	const desdeY = inicio.clientY;

	const mover = (evento: PointerEvent) => {
		alMover(
			Math.round((evento.clientX - desdeX) / paso),
			Math.round((evento.clientY - desdeY) / paso)
		);
	};

	const soltar = () => {
		limpiar();
		emit('commit');
	};

	// `pointercancel` importa: si el sistema o el navegador cancelan el gesto
	// —un gesto del touchpad, la ventana que pierde el foco— `pointerup` no
	// llega nunca, y sin esto el widget seguía al puntero para siempre.
	window.addEventListener('pointermove', mover);
	window.addEventListener('pointerup', soltar);
	window.addEventListener('pointercancel', soltar);

	soltarActual = () => {
		window.removeEventListener('pointermove', mover);
		window.removeEventListener('pointerup', soltar);
		window.removeEventListener('pointercancel', soltar);
		dragging.value = false;
		resizing.value = false;
		soltarActual = null;
	};
}

/**
 * Cómo se sueltan las escuchas del arrastre en curso.
 *
 * Se guarda aparte para poder cortarlo también al desmontar: las escuchas
 * viven en la ventana, así que sobrevivían al componente y quedaban moviendo
 * un widget que ya no existe.
 */
let soltarActual: (() => void) | null = null;

function limpiar() {
	soltarActual?.();
}

onUnmounted(limpiar);

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
		@pointerdown="empezarArrastre"
	>
		<!--
			El marco de todos los widgets vive acá y no en cada uno: fondo,
			borde, esquinas y sombra. Es lo que flota sobre el fondo de
			pantalla, así que va con la superficie opaca `ui-float`, el canto
			fino, el radio `l` y la sombra `surface-m` de la librería; el
			desenfoque que tenía no veía nada detrás. Repetirlo en cada componente era lo que hacía que
			cada widget tuviera su propia opacidad y su propio blur —o ninguno—,
			y que agregar uno nuevo empezara con la pregunta de qué clases copiar.

			`container-type: size` también va acá, así lo de adentro puede medirse
			contra su celda sin que cada widget tenga que declararlo.

			El contenido no recibe clics mientras se edita: si los recibiera,
			arrastrar el reproductor de música cambiaría de canción.
		-->
		<div
			style="container-type: size"
			class="h-full w-full overflow-hidden rounded-corner-l border border-ui-line bg-ui-float shadow-surface-m"
			:class="editing ? 'pointer-events-none select-none' : ''"
		>
			<slot />
		</div>

		<template v-if="editing">
			<div
				class="pointer-events-none absolute inset-0 rounded-corner-l border-2 border-dashed border-primary/70"
			></div>

			<!-- El botón de la librería, con la cruz del tema y no una «×» escrita:
			     el envoltorio es el que corta el arrastre, porque el botón no
			     declara `pointerdown`. -->
			<span class="absolute -right-2 -top-2" @pointerdown.stop>
				<ActionButton
					label=""
					icon="window-close"
					:icon-alt="t('widgets.remove')"
					:title="t('widgets.remove')"
					variant="danger"
					size="sm"
					stop-propagation
					@click="emit('remove')"
				/>
			</span>

			<!-- La manija va abajo a la derecha, que es donde la busca todo el mundo. -->
			<div
				class="absolute -bottom-1 -right-1 h-5 w-5 cursor-nwse-resize rounded-tl-corner-m border-b-2 border-r-2 border-primary bg-ui-float"
				@pointerdown="empezarRedimensionado"
			></div>
		</template>
	</div>
</template>
