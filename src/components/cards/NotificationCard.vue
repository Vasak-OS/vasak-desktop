<template>
  <!-- Una notificación ocupa lo que dice y nada más: sin degradados propios,
       sin sombra, sin levantarse al pasar el mouse y sin colores fuera de los
       del sistema. Lo único que distingue a una sin leer es la barra al
       costado, en el color de acento. El nombre de la aplicación no se repite
       acá: ya está en el encabezado del grupo. -->
  <div
    class="theme-transition group/nc flex items-start gap-2 px-2 py-1.5 border-l-2 border-transparent transition-colors duration-200 hover:bg-ui-surface/60"
    :class="{
      'opacity-60': notification.seen,
      'border-l-primary': !notification.seen,
      'cursor-pointer': hasDefaultAction,
    }" :data-urgency="notification.urgency?.toLowerCase()"
    :title="hasDefaultAction ? t('components.NotificationCard.openHint') : undefined"
    @click="handleDefaultAction">
    <img :src="iconSrc" :alt="notification.app_name" class="w-4 h-4 mt-0.5 shrink-0 object-contain" />
    <div class="flex-1 min-w-0">
      <div class="flex items-start justify-between gap-2">
        <h3 class="text-sm font-medium text-tx-main truncate">{{ notification.summary }}</h3>
        <div class="flex items-center gap-1 shrink-0">
          <span class="text-[11px] text-tx-muted">{{ formatTime(notification.timestamp) }}</span>
          <button @click.stop="$emit('seen', notification.id)"
            :title="t('common.close')"
            class="flex items-center justify-center w-4 h-4 rounded-full text-tx-muted opacity-0 transition-opacity duration-200 group-hover/nc:opacity-100 focus-visible:opacity-100 hover:text-status-error" :aria-label="t('common.close')">
            <img :src="closeIconSrc" :alt="t('common.close')" class="w-2.5 h-2.5" />
          </button>
        </div>
      </div>
      <p v-if="notification.body" class="text-xs text-tx-muted line-clamp-2">
        {{ notification.body }}
      </p>

      <!-- Actions -->
      <div v-if="parsedActions.length > 0" class="flex flex-wrap gap-1 mt-1" @click.stop>
        <ActionButton
          v-for="action in parsedActions"
          :key="action.key"
          :label="action.label"
          variant="secondary"
          custom-class="text-xs"
          @click="() => handleAction(action.key)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed } from 'vue';
import { invokeNotificationAction } from '@/services/notification.service';
import { useIcons } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';
import ActionButton from '../buttons/ActionButton.vue';

const { t } = useI18n();

const props = defineProps<{
	notification: {
		id: number;
		app_name: string;
		app_icon: string;
		summary: string;
		body: string;
		timestamp: number;
		seen: boolean;
		urgency?: string;
		actions?: string[];
		hints?: { [key: string]: string };
	};
}>();

defineEmits<{
	seen: [id: number];
}>();

const { iconSrc, closeIconSrc } = useIcons({
	iconSrc: computed(() => props.notification.app_icon),
	closeIconSrc: 'window-close-symbolic',
});

/**
 * La acción que se ejecuta al hacer clic en la notificación misma.
 *
 * Es la que usan las aplicaciones para «abrí esto»: el navegador para ir a la
 * página, el gestor de archivos para mostrar la descarga. No se dibuja como
 * botón —el botón es la notificación entera— y por eso quedaba inalcanzable:
 * la tarjeta no tenía clic y la lista de botones la salteaba.
 */
const DEFAULT_ACTION = 'default';

const hasDefaultAction = computed(() =>
	(props.notification.actions || []).some(
		(value, index) => index % 2 === 0 && value === DEFAULT_ACTION
	)
);

async function handleDefaultAction() {
	if (!hasDefaultAction.value) return;
	await handleAction(DEFAULT_ACTION);
}

// Parse standard DBus actions [key, label, key, label...]
const parsedActions = computed(() => {
	const acts = props.notification.actions || [];
	const result = [];
	for (let i = 0; i < acts.length; i += 2) {
		const key = acts[i];
		const label = acts[i + 1] || key;
		if (key !== 'default') {
			result.push({ key, label });
		}
	}
	return result;
});

function formatTime(timestamp: number): string {
	const date = new Date(timestamp * 1000);
	return date.toLocaleTimeString();
}

async function handleAction(action_key: string) {
	try {
		await invokeNotificationAction({
			id: props.notification.id,
			action_key,
		});
	} catch (error) {
		logError('Error ejecutando acción de notificación:', error);
	}
}
</script>

