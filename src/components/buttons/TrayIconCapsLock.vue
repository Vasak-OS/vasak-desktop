<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useEventListener } from '@/tools/event.listener';

const { t } = useI18n();

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
    :tooltip="t('components.TrayIconCapsLock.capsLock')"
    :alt="t('components.TrayIconCapsLock.capsLock')"
    :interactive="false"
  />
</template>
