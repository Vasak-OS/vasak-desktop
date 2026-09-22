<script setup lang="ts">
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import type { TrayItem } from '@/interfaces/tray';

const props = defineProps<{
	item: TrayItem;
}>();

/**
 * Three sources, in the order the spec puts them.
 *
 * `IconPixmap` es el mapa de bits de la propia aplicación y gana siempre: es lo
 * que la aplicación dibujó, no algo que se le parece. `IconName` necesita el
 * tema de iconos, y de eso se encarga `ThemeIcon` — sabe cuál está puesto y
 * vuelve a resolver cuando cambia, cosa que la lista fija de seis rutas bajo
 * `hicolor` que tenía el panel no hacía.
 *
 * When neither resolves there is still an item to click, so it gets the initial
 * of its name rather than a blank space. That last case is real: Arch-Update asks
 * for `cachy-update_updates-available-blue`, which is not installed under any
 * name, and an empty square gave no hint that anything was there.
 */
const themeName = computed(() => props.item.icon_name ?? '');

/** El mapa de bits propio, si lo mandó. */
const mapaDeBits = computed(() =>
	props.item.icon_data ? `data:image/png;base64,${props.item.icon_data}` : ''
);

const initial = computed(() => {
	const source = props.item.title || props.item.id || props.item.service_name;
	const letter = source.match(/\p{L}|\p{N}/u);
	return letter ? letter[0].toUpperCase() : '?';
});
</script>

<template>
  <img
    v-if="mapaDeBits"
    :src="mapaDeBits"
    :alt="item.title || item.id"
    class="w-4 h-4 object-contain transition-all duration-300 group-hover:brightness-110 group-hover:scale-110 drop-shadow-[0_1px_2px_rgba(0,0,0,0.3)]"
  />
  <ThemeIcon
    v-else-if="themeName"
    :name="themeName"
    :size="16"
    :alt="item.title || item.id"
    class="object-contain transition-all duration-300 group-hover:brightness-110 group-hover:scale-110 drop-shadow-[0_1px_2px_rgba(0,0,0,0.3)]"
  />
  <span
    v-else
    aria-hidden="true"
    class="grid w-4 h-4 place-items-center rounded-corner bg-ui-surface text-[0.625rem] font-semibold leading-none text-primary transition-all duration-300 group-hover:scale-110"
  >
    {{ initial }}
  </span>
</template>
