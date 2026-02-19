<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import { WindowFrame } from '@vasakgroup/vue-libvasak';
import ConfigSidebarArea from '@/components/areas/configuration/ConfigSidebarArea.vue';
</script>

<template>
  <WindowFrame>
    <div class="h-[calc(100vh-32px)] w-screen rounded-b-window rounded-t-window background">
      <div class="h-full w-full flex p-4 gap-4">
        <ConfigSidebarArea />
        <div class="flex-1 min-h-0">
          <div class="h-full overflow-auto">
            <slot />
          </div>
        </div>
      </div>
    </div>
  </WindowFrame>
</template>
