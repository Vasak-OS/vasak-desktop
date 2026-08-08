<script lang="ts" setup>
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
</script>

<template>
  <transition-group
    tag="div"
    move-class="transition-transform duration-300 ease-out" enter-active-class="transition-all duration-500 ease-out" leave-active-class="transition-all duration-400 ease-in" enter-from-class="opacity-0 scale-80 translate-y-[20px]" leave-to-class="opacity-0 scale-90 -translate-x-[10px]"
    appear
    class="flex flex-wrap gap-1 p-0.5"
  >
    <AppMenuButton 
      v-for="(app, index) in apps" 
      :key="(app as any).name" 
      :app="app as any" 
      :selected="index === selectedIndex"
      class="transition-all rounded-corner hover:border hover:border-secondary duration-300 ease-out hover:scale-[1.02] hover:bg-primary"
    />
  </transition-group>
</template>

