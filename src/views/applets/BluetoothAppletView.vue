
<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import BluetoothControlArea from '@/components/areas/bluetooth/BluetoothControlArea.vue';
import AppletFrame from '@/components/layouts/AppletFrame.vue';
import { toggleBluetoothApplet } from '@/services/window.service';
</script>

<template>
  <AppletFrame :close-fn="toggleBluetoothApplet">
    <BluetoothControlArea />
  </AppletFrame>
</template>
