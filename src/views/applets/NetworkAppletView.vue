<script lang="ts" setup>
/**
 * La vista radial y la lista (vasak-desktop#132): abre en la órbita de la
 * red, con el selector para pasar a Bluetooth.
 */
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import ConnectionsArea from '@/components/areas/connections/ConnectionsArea.vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
</script>

<template>
  <AppletPopover applet="network">
    <ConnectionsArea initial-tab="wifi" applet="network" />
  </AppletPopover>
</template>
