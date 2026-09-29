<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import BluetoothControlArea from '@/components/areas/bluetooth/BluetoothControlArea.vue';
import AppletPopover from '@/components/layouts/AppletPopover.vue';
</script>

<template>
  <AppletPopover applet="bluetooth">
    <BluetoothControlArea />
  </AppletPopover>
</template>
