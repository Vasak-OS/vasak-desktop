<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import { useEventListener } from '@/tools/event.listener';

const { t } = useI18n();

const micMuted = ref(false);

useEventListener<{ active: boolean }>('mic-mute-changed', (event) => {
	micMuted.value = event.payload.active;
});
</script>

<template>
  <TrayIconButton
    v-if="micMuted"
    name="microphone-sensitivity-muted"
    :tooltip="t('components.TrayIconMicrophone.muted')"
    :alt="t('components.TrayIconMicrophone.muted')"
    :interactive="false"
  />
</template>
