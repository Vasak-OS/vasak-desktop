<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useEventListener } from '@/tools/event.listener';

const { t } = useI18n();

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
    :tooltip="t('components.TrayIconMicrophone.muted')"
    :alt="t('components.TrayIconMicrophone.muted')"
    :interactive="false"
  />
</template>
