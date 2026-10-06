<script setup lang="ts">
/**
 * El bloque de ajustes del estado B del centro de control (vasak-desktop#175).
 *
 * Mosaicos en dos columnas —una sola si el bloque es angosto, por consulta de
 * contenedor— y, al tocar uno que tiene detalle, su ficha **dentro del bloque**
 * con «volver»: lista → ficha, como una aplicación de teléfono (decisión 8). Las
 * notificaciones de arriba no se tocan.
 *
 * Los mosaicos quedan montados mientras se ve una ficha (`v-show`), así no
 * vuelven a pedir su estado al volver. La ficha sí se monta y desmonta: lo que
 * escucha y consulta mientras está abierta se va con ella.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton } from '@vasakgroup/vue-libvasak';
import { computed, nextTick, ref } from 'vue';
import { availableTiles, CONTROL_CENTER_TILES, type TileId } from '@/tools/control-center-tiles';

const props = withDefaults(defineProps<{ bluetooth?: boolean }>(), { bluetooth: false });

/** El mosaico cuya ficha está abierta, o `null` con la lista a la vista. */
const detail = defineModel<TileId | null>('detail', { default: null });

const { t } = useI18n();

const tiles = computed(() => availableTiles(CONTROL_CENTER_TILES, { bluetooth: props.bluetooth }));
const openTile = computed(
	() => tiles.value.find((tile) => tile.id === detail.value && tile.detail) ?? null
);

const backButton = ref<HTMLElement | null>(null);
const grid = ref<HTMLElement | null>(null);

/** Abre la ficha y lleva el foco a «volver», que es lo primero de la ficha. */
async function open(id: TileId): Promise<void> {
	detail.value = id;
	await nextTick();
	backButton.value?.querySelector('button')?.focus();
}

/** Vuelve a la lista con el foco en la flecha que abrió la ficha. */
async function back(): Promise<void> {
	const from = detail.value;
	detail.value = null;
	await nextTick();
	grid.value?.querySelector<HTMLElement>(`[data-tile-id="${from}"] [data-tile-detail]`)?.focus();
}
</script>

<template>
  <section id="control-center-quick-settings" class="@container flex flex-col gap-2" data-quick-settings>
    <!-- Ni la lista ni la ficha se achican por debajo de lo suyo: si no
         entran, se desplaza el bloque de arriba del centro entero, que es un
         solo desplazamiento y no uno adentro de otro. Cada mosaico es un
         `QuickSettingsTile` de la librería: la tarjeta la pone él. -->
    <!-- `fieldset` y no `role="group"`: el grupo nativo lo anuncian todos los
         lectores. `min-w-0` le saca el ancho mínimo de contenido que trae. -->
    <fieldset
      v-show="!openTile"
      ref="grid"
      :aria-label="t('views.controlCenter.quickSettings')"
      class="m-0 grid min-w-0 shrink-0 grid-cols-1 gap-2 border-0 p-0 @[17rem]:grid-cols-2"
      data-tile-grid
    >
      <div v-for="spec in tiles" :key="spec.id" class="min-w-0" :data-tile-id="spec.id">
        <component :is="spec.tile" @open="open(spec.id)" />
      </div>
    </fieldset>

    <template v-if="openTile">
      <div ref="backButton" class="flex shrink-0">
        <ActionButton
          :label="t('common.back')"
          icon="go-previous"
          icon-type="symbol"
          variant="ghost"
          size="sm"
          data-tile-back
          @click="back"
        />
      </div>
      <div class="flex shrink-0 flex-col" :data-tile-sheet="openTile.id">
        <component :is="openTile.detail" v-bind="openTile.detailProps ?? {}" />
      </div>
    </template>
  </section>
</template>
