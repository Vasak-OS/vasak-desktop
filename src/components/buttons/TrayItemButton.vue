<script setup lang="ts">
import { ThemeIcon } from '@vasakgroup/vue-libvasak';
import { computed } from 'vue';
import TrayPixmap from '@/components/buttons/TrayPixmap.vue';
import type { TrayItem } from '@/interfaces/tray';
import { itemName, mainIcon, overlayIcon } from '@/tools/tray-item';

const props = defineProps<{
	item: TrayItem;
}>();

/**
 * El icono del elemento y, si la manda, su insignia superpuesta.
 *
 * El icono sale de `mainIcon` (`tools/tray-item.ts`): el mapa de bits de la
 * aplicación gana porque es lo que ella dibujó; el nombre va por `ThemeIcon`,
 * que sabe cuál tema está puesto y vuelve a resolver cuando cambia. Con
 * `NeedsAttention`, el icono de atención si lo hay.
 *
 * Cuando no hay ni uno ni otro todavía hay un elemento que tocar, así que va
 * la inicial de su nombre y no un hueco. Ese caso existe: Arch-Update pide
 * `cachy-update_updates-available-blue`, que no está instalado con ningún
 * nombre, y un cuadrado vacío no decía que ahí hubiera algo.
 *
 * La insignia (`OverlayIconName` / `OverlayIconPixmap`) va en la esquina de
 * abajo a la derecha, a la mitad del tamaño, como la dibujan KDE y GNOME. Sin
 * insignia no hay nada en esa esquina.
 */
const icon = computed(() => mainIcon(props.item));
const overlay = computed(() => overlayIcon(props.item));
const name = computed(() => itemName(props.item));

const initial = computed(() => {
	const letter = name.value.match(/\p{L}|\p{N}/u) ?? props.item.service_name.match(/\p{L}|\p{N}/u);
	return letter ? letter[0].toUpperCase() : '?';
});
</script>

<template>
  <span class="relative inline-flex size-4 shrink-0 items-center justify-center">
    <TrayPixmap
      v-if="icon?.kind === 'pixmap'"
      :data="icon.data"
      :size="16"
      :alt="name"
      class="transition-[filter] duration-200 ease-ui group-hover:brightness-110"
    />
    <ThemeIcon
      v-else-if="icon?.kind === 'theme'"
      :name="icon.name"
      :size="16"
      :alt="name"
      class="object-contain transition-[filter] duration-200 ease-ui group-hover:brightness-110"
    />
    <span
      v-else
      aria-hidden="true"
      class="grid size-4 place-items-center rounded-corner-m bg-ui-surface text-[0.625rem] font-semibold leading-none text-primary"
    >
      {{ initial }}
    </span>

    <span
      v-if="overlay"
      data-tray-overlay
      aria-hidden="true"
      class="absolute -right-0.5 -bottom-0.5 inline-flex size-2.5 items-center justify-center"
    >
      <TrayPixmap v-if="overlay.kind === 'pixmap'" :data="overlay.data" :size="10" />
      <ThemeIcon v-else :name="overlay.name" :size="10" alt="" />
    </span>
  </span>
</template>
