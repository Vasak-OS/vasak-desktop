<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import TrayIconButton from '@/components/buttons/TrayIconButton.vue';
import {
	getTwingateInfo,
	type TwingateInfo,
	toggleTwingateApplet,
} from '@/services/twingate.service';
import { useSymbol } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';
import { logError } from '@/utils/logger';

/**
 * Twingate en el panel: un icono, y nada de texto.
 *
 * Antes había una etiqueta de hasta 176 píxeles con el nombre del perfil al lado
 * del icono de red, que repetía en la franja más escasa de la pantalla algo que
 * ya estaba en el tooltip. Acá el nombre va en el tooltip y el detalle —los
 * recursos, sus vencimientos, qué falta autorizar— en el applet.
 *
 * El botón sólo existe si el cliente está instalado: un icono permanente para
 * algo que la mayoría de las máquinas no usa es basura en el panel.
 */
const { t } = useI18n();

const info = ref<TwingateInfo | null>(null);

const icon = useSymbol(
	computed(() => (info.value?.connected ? 'network-vpn' : 'network-vpn-disconnected'))
);

const cargar = async () => {
	try {
		info.value = await getTwingateInfo();
	} catch (error) {
		logError('[twingate] No se pudo leer el estado:', error);
		info.value = null;
	}
};

onMounted(cargar);

// Lo que mueve este icono es que Twingate se conecte o se caiga, y eso llega
// como un cambio de VPN: la interfaz `tun` aparece o desaparece.
useSharedEvent('vpn-changed', cargar);

const porAutorizar = computed(
	() => info.value?.resources.filter((recurso) => recurso.needs_auth).length ?? 0
);

const tooltip = computed(() => {
	if (!info.value?.connected) return t('components.TrayIconTwingate.disconnected');
	if (porAutorizar.value > 0) {
		return t('components.TrayIconTwingate.pending').replace('{0}', String(porAutorizar.value));
	}

	return t('components.TrayIconTwingate.connected').replace(
		'{0}',
		String(info.value.resources.length)
	);
});
</script>

<template>
  <TrayIconButton
    v-if="info?.installed"
    :icon="icon"
    :alt="tooltip"
    :tooltip="tooltip"
    :custom-class="{ relative: true }"
    :icon-class="{ 'filter brightness-90': !info.connected }"
    @click="toggleTwingateApplet"
  >
    <!-- Un punto y no un número: lo que hace falta saber de un vistazo es que
         hay algo que autorizar; cuántos, lo dice el applet. -->
    <div
      v-if="porAutorizar > 0"
      class="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-status-warning ring-1 ring-ui-bg"
    ></div>
  </TrayIconButton>
</template>
