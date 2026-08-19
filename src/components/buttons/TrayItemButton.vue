<script setup lang="ts">
import { computed } from 'vue';
import type { TrayItem } from '@/interfaces/tray';
import { useIcon } from '@/tools/composables/useReactiveIcon';

const props = defineProps<{
	item: TrayItem;
}>();

/**
 * Three sources, in the order the spec puts them.
 *
 * `IconPixmap` is the app's own bitmap and always wins: it is what the app drew,
 * not something that looks like it. `IconName` needs the icon theme, and that is
 * what `useIcon` is for — it knows the active theme and reloads when it changes,
 * which the panel's previous hardcoded list of six paths under `hicolor` did not.
 *
 * When neither resolves there is still an item to click, so it gets the initial
 * of its name rather than a blank space. That last case is real: Arch-Update asks
 * for `cachy-update_updates-available-blue`, which is not installed under any
 * name, and an empty square gave no hint that anything was there.
 */
const themeName = computed(() => props.item.icon_name ?? '');
const themeIcon = useIcon(themeName);

const iconSrc = computed(() => {
	if (props.item.icon_data) return `data:image/png;base64,${props.item.icon_data}`;
	return themeName.value ? themeIcon.value : '';
});

const initial = computed(() => {
	const source = props.item.title || props.item.id || props.item.service_name;
	const letter = source.match(/\p{L}|\p{N}/u);
	return letter ? letter[0].toUpperCase() : '?';
});
</script>

<template>
  <img
    v-if="iconSrc"
    :src="iconSrc"
    :alt="item.title || item.id"
    class="w-4 h-4 object-contain transition-all duration-300 group-hover:brightness-110 group-hover:scale-110 drop-shadow-[0_1px_2px_rgba(0,0,0,0.3)]"
  />
  <span
    v-else
    aria-hidden="true"
    class="grid w-4 h-4 place-items-center rounded-corner bg-ui-surface text-[0.625rem] font-semibold leading-none text-primary transition-all duration-300 group-hover:scale-110"
  >
    {{ initial }}
  </span>
</template>
