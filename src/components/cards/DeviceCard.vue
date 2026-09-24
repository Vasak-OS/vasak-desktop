<template>
  <!--
    La fila entera se abre con el mouse **y** con el teclado, pero sólo cuando
    de verdad hace algo. `role="button"` y no un `<button>` de verdad porque
    adentro hay otro botón —el de conectar— y un botón dentro de otro no es
    HTML válido: el navegador desanida el marcado y el de adentro deja de
    funcionar.

    Y va atado a `clickable`, que estaba declarado y no se usaba: `handleClick`
    emitía siempre, nadie escuchaba ese `click` y la fila no tenía ni el cursor
    que lo anunciara. Una fila que dice ser un botón y no hace nada es peor que
    una que no lo dice.
  -->
  <div
    class="flex items-center justify-between bg-ui-bg/80 rounded-corner border border-ui-border px-6 py-3 mb-4"
    :class="[
      { 'border-l-4 border-status-success': isConnected, 'cursor-pointer': clickable },
      customClass,
    ]"
    :role="clickable ? 'button' : undefined"
    :tabindex="clickable ? 0 : undefined"
    :aria-label="clickable ? title : undefined"
    @click="handleClick"
    @keydown.enter.prevent="handleClick"
    @keydown.space.prevent="handleClick"
  >
    <div class="flex items-center gap-3 flex-1 min-w-0">
      <ThemeIcon :name="icon" :size="28" :alt="title" />
      <div class="min-w-0">
        <div class="font-semibold truncate">
          {{ title }}
        </div>
        <div v-if="subtitle" class="text-xs text-tx-muted truncate">
          {{ subtitle }}
        </div>
        <div v-if="metadata" class="text-xs text-tx-muted truncate">
          {{ metadata }}
        </div>
        <div v-if="extraInfo && extraInfo.length > 0" class="text-xs text-tx-muted flex gap-3 mt-1 flex-wrap">
          <span v-for="(info, index) in extraInfo" :key="index" class="inline-flex items-center gap-1">
            <ThemeIcon :name="info.icon" type="symbol" :size="14" />
            {{ info.text }}
          </span>
        </div>
      </div>
    </div>
    
    <button
      v-if="showActionButton"
      class="bg-vsk-primary rounded-vsk px-4 py-2 text-sm font-semibold cursor-pointer hover:opacity-70 disabled:opacity-50 disabled:cursor-not-allowed"
      :disabled="isConnecting"
      @click.stop="handleAction"
    >
      {{ isConnecting ? t('components.DeviceCard.connecting') : (actionLabel || t('components.DeviceCard.connect')) }}
    </button>

    <!-- Status indicator for connected state -->
    <div
      v-if="showStatusIndicator && isConnected"
      class="w-2 h-2 rounded-full bg-status-success"
    />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ThemeIcon } from '@vasakgroup/vue-libvasak';

const { t } = useI18n();

interface ExtraInfoItem {
	icon: string;
	text: string;
}

interface Props {
	icon: string;
	title: string;
	subtitle?: string;
	metadata?: string;
	extraInfo?: ExtraInfoItem[];
	isConnected?: boolean;
	isConnecting?: boolean;
	showActionButton?: boolean;
	actionLabel?: string;
	showStatusIndicator?: boolean;
	customClass?: string;
	clickable?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
	subtitle: '',
	metadata: '',
	extraInfo: () => [],
	isConnected: false,
	isConnecting: false,
	showActionButton: true,
	actionLabel: '',
	showStatusIndicator: false,
	customClass: '',
	clickable: false,
});

const emit = defineEmits<{
	action: [];
	click: [];
}>();

const handleAction = () => {
	emit('action');
};

const handleClick = () => {
	if (!props.clickable) return;
	emit('click');
};
</script>
