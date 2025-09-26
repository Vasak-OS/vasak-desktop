<script lang="ts" setup>
import ConfigSidebarArea from "@/components/areas/configuration/ConfigSidebarArea.vue";
import { WindowFrame } from "@vasakgroup/vue-libvasak";
</script>
<template>
  <WindowFrame class="rounded-t-window">
    <div class="h-[calc(100vh-32px)] rounded-b-window background">
      <div class="h-full w-full flex flex-wrap p-4 gap-4">
        <ConfigSidebarArea />
        <slot />
      </div>
    </div>
  </WindowFrame>
</template>
