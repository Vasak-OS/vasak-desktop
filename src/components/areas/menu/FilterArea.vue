<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: imports used in template */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import AppMenuButton from '@/components/buttons/AppMenuButton.vue';

// Receives the already-filtered list rather than filtering again.
//
// It used to take the full list plus the raw query and filter it itself, while
// MenuView computed its own filtered list for keyboard navigation. The two
// disagreed: this one compared a lowercased name against the *un*-lowercased
// query, so any capital letter matched nothing — typing "F" for Firefox gave an
// empty menu — and the arrow-key highlight indexed into a different list than
// the one on screen.
defineProps({
	apps: {
		type: Array,
		required: true,
	},
	selectedIndex: {
		type: Number,
		default: 0,
	},
});

const { t } = useI18n();
</script>

<template>
  <!-- Sin `transition-group`: cada resultado entraba escalando en 500 ms y se
       iba corriéndose. Once UI anima una sola cosa, el panel; la lista cambia
       con lo que se escribe y tiene que seguirle el ritmo. -->
  <div role="menu" :aria-label="t('views.menu.results')" class="flex flex-wrap gap-1 p-0.5">
    <AppMenuButton
      v-for="(app, index) in apps"
      :key="(app as any).name"
      :app="app as any"
      :selected="index === selectedIndex"
    />
  </div>
</template>
