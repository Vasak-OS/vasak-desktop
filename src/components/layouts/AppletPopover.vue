<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { type AppletId, dismissApplet } from '@/services/window.service';
import {
	type AppletAnchor,
	type AppletInset,
	type AppletShownEvent,
	anchorFromQuery,
	appletTransformOrigin,
	fullFromQuery,
	insetFromQuery,
	insetStyle,
	toFull,
	toInset,
} from '@/tools/applet-anchor';
import { useSharedEvent } from '@/tools/event.bus';
import { logWarning } from '@/utils/logger';

/**
 * El contenedor de un applet del panel.
 *
 * Reemplaza a `AppletFrame`: lo que hace es lo mismo —el borde, el fondo, la
 * animación, cerrarse con Escape—, pero ahora el applet cuelga del botón que lo
 * abrió y la entrada **crece desde el botón**.
 *
 * La forma es la de algo que sale del panel en Once UI (vue-libvasak#74,
 * §5.3): `rounded-corner-xl`, el canto fino `ui-line`, la superficie translúcida
 * `ui-shell` y la sombra `surface-l`, todo de `tokens.css`. Translúcida y sin
 * desenfoque propio: el del escritorio lo pone Wayfire detrás de la superficie
 * de capa, y una superficie opaca lo taparía (corrección del 02/10/2026; el
 * WebView no ve el escritorio, así que desenfocar desde la página no
 * desenfocaba nada). La entrada dura 200 ms con `ease-ui-out` —arranca rápido y frena—, y la salida 120.
 *
 * El backend dice de qué lado está el panel y dónde quedó el botón a lo largo
 * del applet (`applet-anchor.ts`); la primera vez por la ruta, después por
 * `applet-shown`.
 *
 * La superficie es más grande que el applet —el margen de sombra de
 * `anchored_applet.rs`, para que la sombra no se corte en el canto—: el applet
 * se dibuja a `inset` de cada borde, y lo de alrededor queda transparente. Los
 * clics sobre ese margen no llegan acá: el backend recorta la región de entrada
 * de la superficie al applet, y caen en lo que haya debajo, que suele ser el
 * panel.
 *
 * Quien cierra es el backend. Escape y la pérdida de foco los atrapa la
 * superficie de capa antes que la página, y avisa con `applet-leave` para que la
 * salida se vea; la página sólo pide cerrar cuando termina lo suyo (`close`, que
 * se le pasa al contenido).
 *
 * Emite `shown` cada vez que el applet vuelve a la vista: esconder no destruye
 * el webview, así que Vue no se monta de nuevo y el contenido tiene que volver a
 * pedir lo que muestra. Y `leave` cuando se empieza a ir, para que corte lo que
 * tenga en curso —el menú, sus reintentos de foco—.
 */
const props = defineProps<{
	applet: AppletId;
	/**
	 * Para un menú: el relleno de un menú contextual (`p-1`) en lugar del de un
	 * applet (`p-4`). El backend cuenta con esa medida al calcular el alto del
	 * menú de la bandeja (`tray_menu_size` en `commands/tray.rs`).
	 */
	compact?: boolean;
}>();

const emit = defineEmits<{
	shown: [];
	leave: [];
}>();

// Lo que se le pase desde afuera —una clase, un `aria-*`— va al applet que se
// ve y no al envoltorio transparente del margen de sombra.
defineOptions({ inheritAttrs: false });

const route = useRoute();
const anchor = ref<AppletAnchor | null>(anchorFromQuery(route.query));
const inset = ref<AppletInset>(insetFromQuery(route.query));
/**
 * Si es el overlay a pantalla completa del menú (el modo `full` de
 * `menu_display.rs`): sin borde, sin cantos redondeados ni sombra —no hay afuera
 * donde se vean— y con un fundido en lugar de crecer desde el botón, que es cosa
 * del applet anclado.
 */
const full = ref<boolean>(fullFromQuery(route.query));

/**
 * En qué momento de la vida del applet estamos.
 *
 * `hidden` es invisible sin animar: lo que queda puesto mientras la superficie
 * está escondida, para que al volver a mostrarse no asome el último cuadro antes
 * de la entrada.
 */
const phase = ref<'enter' | 'leave' | 'hidden'>('enter');

const transformOrigin = computed(() => appletTransformOrigin(anchor.value));
const placement = computed(() => ({
	...insetStyle(inset.value),
	transformOrigin: transformOrigin.value,
}));

const close = async () => {
	try {
		await dismissApplet(props.applet);
	} catch (error) {
		logWarning(`[AppletPopover] no se pudo cerrar ${props.applet}:`, error);
	}
};

useSharedEvent<AppletShownEvent>('applet-shown', (payload) => {
	if (payload?.applet !== props.applet) return;

	anchor.value = { side: payload.side, origin: payload.origin };
	inset.value = toInset(payload.inset);
	full.value = toFull(payload.full);
	// Invisible un cuadro y recién después la entrada: poner `enter` sobre
	// `enter` no vuelve a correr la animación.
	phase.value = 'hidden';
	requestAnimationFrame(() => {
		phase.value = 'enter';
	});
	emit('shown');
});

useSharedEvent<{ applet: string; instant: boolean }>('applet-leave', (payload) => {
	if (payload?.applet !== props.applet) return;
	phase.value = payload.instant ? 'hidden' : 'leave';
	emit('leave');
});

// Por si Escape llega a la página: la superficie lo atrapa antes, pero un
// webview con el foco en un campo de texto puede quedárselo.
const onKeydown = (event: KeyboardEvent) => {
	if (event.key === 'Escape') void close();
};

onMounted(() => document.addEventListener('keydown', onKeydown));
onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown));
</script>

<template>
  <div class="relative h-screen w-screen">
    <div
      role="dialog"
      :class="[
        'applet-popover absolute overflow-hidden bg-ui-shell',
        // El overlay a pantalla completa no lleva borde, cantos redondeados ni
        // sombra: no hay afuera donde se vean. El applet anclado sí, con la forma
        // de lo que flota (vue-libvasak#74, §5.3).
        full ? 'applet-popover-full' : 'rounded-corner-xl border border-ui-line shadow-surface-l',
        compact ? 'p-1' : 'p-4',
        `applet-popover-${phase}`,
      ]"
      :style="placement"
      v-bind="$attrs"
    >
      <slot :close="close" />
    </div>
  </div>
</template>

<style scoped>
@keyframes applet-popover-in {
  from {
    opacity: 0;
    transform: scale(0.96);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

@keyframes applet-popover-out {
  from {
    opacity: 1;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(0.96);
  }
}

@keyframes applet-popover-fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes applet-popover-fade-out {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}

.applet-popover-enter {
  animation: applet-popover-in 200ms var(--ease-ui-out) both;
}

/* La salida, al revés y más corta. `forwards` la deja invisible hasta que el
   backend esconde la superficie. */
.applet-popover-leave {
  animation: applet-popover-out 120ms ease-in forwards;
}

.applet-popover-hidden {
  opacity: 0;
}

/* El overlay a pantalla completa entra y sale con un fundido, no creciendo desde
   el botón: crecer es del applet anclado. La duración y el easing los hereda de
   las reglas de arriba; sólo cambia la animación. */
.applet-popover-full.applet-popover-enter {
  animation-name: applet-popover-fade-in;
}

.applet-popover-full.applet-popover-leave {
  animation-name: applet-popover-fade-out;
}

/* Sin movimiento, sólo opacidad. Lo resuelve el CSS al dibujar: `matchMedia`
   no avisa de cambios en este WebView, pero la regla sí se aplica. */
@media (prefers-reduced-motion: reduce) {
  .applet-popover-enter {
    animation-name: applet-popover-fade-in;
  }

  .applet-popover-leave {
    animation-name: applet-popover-fade-out;
  }
}
</style>
