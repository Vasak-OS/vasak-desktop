<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
import ConfigSidebarButton from '@/components/buttons/ConfigSidebarButton.vue';
</script>

<template>
  <div class="w-20 bg-ui-bg/80 h-full rounded-corner flex flex-col items-center p-2 gap-2">
    <ul>
      <ConfigSidebarButton icon="style" to="/apps/configuration/style" title="Style & Theme Settings" />
      <ConfigSidebarButton icon="keyboard" to="/apps/configuration/shortcuts" title="Keyboard Shortcuts" />
      <ConfigSidebarButton icon="network" to="/apps/configuration/network" title="Network Settings" />
    </ul>
  </div>
</template>
