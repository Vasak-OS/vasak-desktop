<script lang="ts" setup>
import ConfigSidebarButton from '@/components/buttons/ConfigSidebarButton.vue';
</script>

<template>
  <div class="w-20 background h-full rounded-vsk flex flex-col items-center p-2 gap-2">
    <ul>
      <ConfigSidebarButton icon="userinfo" to="/apps/configuration/info" title="Information" />
      <ConfigSidebarButton icon="style" to="/apps/configuration/style" title="Style & Theme Settings" />
      <ConfigSidebarButton icon="volume2" to="/apps/configuration/audio" title="Audio Settings" />
      <ConfigSidebarButton icon="keyboard" to="/apps/configuration/shortcuts" title="Keyboard Shortcuts" />
      <ConfigSidebarButton icon="network" to="/apps/configuration/network" title="Network Settings" />
      <ConfigSidebarButton icon="bluetooth" to="/apps/configuration/bluetooth" title="Bluetooth Settings" />
    </ul>
  </div>
</template>
