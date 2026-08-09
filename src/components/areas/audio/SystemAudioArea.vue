<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */

import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import AudioDeviceSelector from '@/components/controls/AudioDeviceSelector.vue';
import VolumeControl from '@/components/controls/VolumeControl.vue';

const { t } = useI18n();
</script>
<template>
  <div class="p-4">
    <h1 class="text-2xl font-bold mb-4">{{ t('components.SystemAudioArea.title') }}</h1>
    <div class="mb-6">
      <h2 class="text-xl font-semibold mb-2">{{ t('components.SystemAudioArea.volumeControl') }}</h2>
      <VolumeControl />
    </div>
    <h2 class="text-xl font-semibold mb-2">{{ t('components.SystemAudioArea.deviceSelector') }}</h2>
    <div class="bg-ui-bg/80 rounded-corner p-4">
      <AudioDeviceSelector />
    </div>
  </div>
</template>
