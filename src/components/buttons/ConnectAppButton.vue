<script setup lang="ts">
import { computed } from 'vue';
import type { ConnectApp } from '@/interfaces/connect';
import { initialFor, themeIconFor } from '@/tools/androidIcon';
import { useIcon } from '@/tools/composables/useReactiveIcon';

const props = defineProps<{
	app: ConnectApp;
	running: boolean;
}>();

const emit = defineEmits<{
	open: [];
	close: [];
}>();

/**
 * The icon is resolved per row with `useIcon`, the same way the application menu
 * resolves each of its entries: the list is built from data, so a fixed map of
 * names at setup — what `useIcons` takes — cannot cover it.
 *
 * An empty name resolves to nothing, which is the case for every app without a
 * mapping. The template shows the initial then.
 */
const themeName = computed(() => themeIconFor(props.app.package) ?? '');
const themeIcon = useIcon(themeName);

/** A path sent by the service wins: it is the app's real icon, not a lookalike. */
const iconSrc = computed(() => props.app.icon || (themeName.value ? themeIcon.value : ''));

const initial = computed(() => initialFor(props.app.label, props.app.package));
</script>

<template>
  <div class="group flex items-center gap-3 rounded-corner p-2 hover:bg-primary/20">
    <button type="button" class="flex min-w-0 flex-1 items-center gap-3 text-left" @click="emit('open')">
      <img v-if="iconSrc" :src="iconSrc" alt="" class="h-8 w-8 shrink-0" />
      <span
        v-else
        aria-hidden="true"
        class="grid h-8 w-8 shrink-0 place-items-center rounded-corner bg-ui-surface font-semibold text-primary"
      >
        {{ initial }}
      </span>
      <span class="truncate">{{ app.label }}</span>
    </button>
    <slot name="actions" />
  </div>
</template>
