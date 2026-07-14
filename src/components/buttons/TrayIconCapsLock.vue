<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { ref } from 'vue';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import { useEventListener } from '@/tools/event.listener';

const capsLockOn = ref(false);

const capsLockIcon = useSymbol('capslock-enabled-symbolic');

useEventListener<{ active: boolean }>('caps-lock-changed', (event) => {
	capsLockOn.value = event.payload.active;
});
</script>

<template>
  <TrayIconButton
    v-if="capsLockOn"
    :icon="capsLockIcon"
    tooltip="Bloq Mayús"
    alt="Caps Lock"
  />
</template>
