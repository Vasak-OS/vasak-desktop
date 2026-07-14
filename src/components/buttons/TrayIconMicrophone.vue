<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { ref } from 'vue';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import { useEventListener } from '@/tools/event.listener';

const micMuted = ref(false);

const micIcon = useSymbol('microphone-sensitivity-muted');

useEventListener<{ active: boolean }>('mic-mute-changed', (event) => {
	micMuted.value = event.payload.active;
});
</script>

<template>
  <TrayIconButton
    v-if="micMuted"
    :icon="micIcon"
    tooltip="Micrófono silenciado"
    alt="Mic Muted"
  />
</template>
