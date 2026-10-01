<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton } from '@vasakgroup/vue-libvasak';

/**
 * El marco de un widget: la caja contra la que se mide lo de adentro.
 *
 * Había dos copias —el de la cuadrícula del escritorio (`WidgetHost`) y el
 * suelto del menú (`WidgetSlot`)— con la misma caja y distinta superficie.
 * Queda acá, propio del escritorio y con los tokens de la librería, como se
 * decidió el 01/10/2026: sube a vue-libvasak cuando aparezca una segunda
 * aplicación con widgets.
 *
 * Lo que sabe:
 *
 * - **`container-type: size`**, para que el widget se mida en unidades de
 *   contenedor (`cqmin`) contra su celda y no contra la ventana entera. Por eso
 *   el contenido de los widgets no usa componentes de la librería, que tienen
 *   medidas fijas (§5 del inventario de vue-libvasak#74).
 * - **Llenar el marco**: el widget ocupa todo, sin relleno propio.
 * - **La superficie**: `float` es lo que flota sobre el fondo de pantalla
 *   (`ui-float` opaca, sombra `surface-m`); `surface` es lo que se apoya dentro
 *   de otra ventana, como el clima del menú (`ui-surface/70`, sin sombra).
 *   Las dos con el canto fino y el radio `l`. Sin desenfoque: detrás de una
 *   superficie de capa no hay nada que desenfocar.
 * - **La edición**: el canto discontinuo, el botón de quitar y el tirador de
 *   abajo a la derecha. Mientras se edita, lo de adentro no recibe clics: si
 *   los recibiera, arrastrar el reproductor cambiaría de canción.
 *
 * Arrastrar y redimensionar no es de acá: el marco avisa con `resize-start`
 * cuando se aprieta el tirador, y quien lo ubica en la cuadrícula hace la
 * cuenta.
 */
withDefaults(
	defineProps<{
		surface?: 'float' | 'surface';
		editing?: boolean;
	}>(),
	{ surface: 'float', editing: false }
);

const emit = defineEmits<{
	remove: [];
	'resize-start': [event: PointerEvent];
}>();

const { t } = useI18n();
</script>

<template>
  <div class="relative h-full w-full min-w-0" data-widget-frame>
    <div
      style="container-type: size"
      class="h-full w-full overflow-hidden rounded-corner-l border border-ui-line"
      :class="[
        surface === 'float' ? 'bg-ui-float shadow-surface-m' : 'bg-ui-surface/70',
        editing ? 'pointer-events-none select-none' : '',
      ]"
    >
      <slot />
    </div>

    <template v-if="editing">
      <div class="pointer-events-none absolute inset-0 rounded-corner-l border-2 border-dashed border-primary/70"></div>

      <!-- El botón de la librería, con la cruz del tema: el envoltorio es el
           que corta el arrastre, porque el botón no declara `pointerdown`. -->
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
        data-resize-handle
        @pointerdown="emit('resize-start', $event)"
      ></div>
    </template>
  </div>
</template>
