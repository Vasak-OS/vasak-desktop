<script setup lang="ts">
/**
 * El micrófono en el estado B del centro de control (vasak-desktop#182): el
 * volumen de entrada con su botón de silenciar y, a la derecha, la flecha ›
 * que abre la elección de la entrada dentro del bloque de ajustes.
 *
 * Es un componente propio y no una variante de `VolumeControl` para que los
 * controles que se suman debajo (brillo por monitor, energía) no tengan que
 * tocar éste. Sin micrófono se ve «no disponible», nunca roto, y la flecha
 * sigue: en la ficha se ve que no hay entradas.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ActionButton, SliderControl, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { onMounted } from 'vue';
import { useMicrophoneState } from '@/tools/composables/useMicrophoneState';

const emit = defineEmits<{ 'open-devices': [] }>();
const { t } = useI18n();
const microphone = useMicrophoneState();
const { available, loaded, isMuted, currentVolume, currentIcon } = microphone;

onMounted(() => {
	void microphone.refresh();
});
</script>

<template>
  <div class="flex w-full min-w-0 items-center gap-1" data-microphone-control>
    <SliderControl
      v-if="available"
      class="min-w-0 flex-1"
      :name="currentIcon"
      type="symbol"
      :label="t('components.MicrophoneControl.microphone')"
      :button-label="isMuted
        ? t('components.MicrophoneControl.unmute')
        : t('components.MicrophoneControl.mute')"
      :model-value="currentVolume"
      :min="0"
      :max="100"
      :show-button="true"
      :icon-class="{ 'opacity-60': isMuted }"
      data-microphone-slider
      @update:model-value="microphone.updateVolume"
      @button-click="microphone.toggleMute"
    />
    <div
      v-else-if="loaded"
      class="flex min-h-8 min-w-0 flex-1 items-center gap-2 px-2 text-label-s text-tx-muted"
      data-microphone-unavailable
    >
      <ThemeIcon name="microphone-sensitivity-muted-symbolic" type="symbol" :size="16" alt="" />
      <span class="truncate">{{ t('components.MicrophoneControl.unavailable') }}</span>
    </div>
    <ActionButton
      label=""
      icon="go-next"
      icon-type="symbol"
      :icon-alt="t('components.MicrophoneControl.devices')"
      :title="t('components.MicrophoneControl.devices')"
      variant="ghost"
      size="sm"
      class="shrink-0"
      data-sheet-opener="audio-input"
      @click="emit('open-devices')"
    />
  </div>
</template>
