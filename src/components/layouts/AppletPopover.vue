<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { type AppletId, dismissApplet } from '@/services/window.service';
import {
	type AppletAnchor,
	type AppletShownEvent,
	anchorFromQuery,
	appletTransformOrigin,
} from '@/tools/applet-anchor';
import { useSharedEvent } from '@/tools/event.bus';
import { logWarning } from '@/utils/logger';

/**
 * El contenedor de un applet del panel.
 *
 * Reemplaza a `AppletFrame`: lo que hace es lo mismo —el borde, el fondo, la
 * animación, cerrarse con Escape—, pero ahora el applet cuelga del botón que lo
 * abrió y la entrada **crece desde el botón**. El backend dice de qué lado está
 * el panel y dónde quedó el botón a lo largo del applet (`applet-anchor.ts`); la
 * primera vez por la ruta, después por `applet-shown`.
 *
 * Quien cierra es el backend. Escape y la pérdida de foco los atrapa la
 * superficie de capa antes que la página, y avisa con `applet-leave` para que la
 * salida se vea; la página sólo pide cerrar cuando termina lo suyo (`close`, que
 * se le pasa al contenido).
 *
 * Emite `shown` cada vez que el applet vuelve a la vista: esconder no destruye
 * el webview, así que Vue no se monta de nuevo y el contenido tiene que volver a
 * pedir lo que muestra.
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
}>();

const route = useRoute();
const anchor = ref<AppletAnchor | null>(anchorFromQuery(route.query));

/**
 * En qué momento de la vida del applet estamos.
 *
 * `hidden` es invisible sin animar: lo que queda puesto mientras la superficie
 * está escondida, para que al volver a mostrarse no asome el último cuadro antes
 * de la entrada.
 */
const phase = ref<'enter' | 'leave' | 'hidden'>('enter');

const transformOrigin = computed(() => appletTransformOrigin(anchor.value));

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
  <div
    role="dialog"
    :class="[
      'applet-popover h-screen w-screen overflow-hidden rounded-corner border border-ui-border bg-ui-bg/80 backdrop-blur-md',
      compact ? 'p-1' : 'p-4',
      `applet-popover-${phase}`,
    ]"
    :style="{ transformOrigin }"
  >
    <slot :close="close" />
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
  animation: applet-popover-in 180ms ease-out both;
}

/* La salida, al revés y más corta. `forwards` la deja invisible hasta que el
   backend esconde la superficie. */
.applet-popover-leave {
  animation: applet-popover-out 120ms ease-in forwards;
}

.applet-popover-hidden {
  opacity: 0;
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
