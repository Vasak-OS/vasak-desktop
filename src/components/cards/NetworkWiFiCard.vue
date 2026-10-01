<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	Dialog,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	ListCard,
	TextInput,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, nextTick, ref } from 'vue';
import {
	connectToWifi,
	type NetworkInfo,
	type WiFiConnectionConfig,
} from '@/services/network.service';

const props = defineProps<NetworkInfo>();
const { t } = useI18n();

const showModal = ref(false);
const password = ref('');
const connecting = ref(false);
const errorMsg = ref('');

const connectToNetwork = async () => {
	if (props.is_connected) return;
	showModal.value = true;
	await nextTick();
};

const confirmConnect = async () => {
	connecting.value = true;
	errorMsg.value = '';
	try {
		await connectToWifi({ ssid: props.ssid, password: password.value } as WiFiConnectionConfig);
		showModal.value = false;
		password.value = '';
	} catch (error) {
		errorMsg.value = t('components.NetworkWiFiCard.connectError').replace(
			'{0}',
			String((error as any)?.message)
		);
	} finally {
		connecting.value = false;
	}
};

const securityLabelMap: Record<string, string> = {
	wep: 'WEP',
	'wpa-psk': 'WPA-PSK',
	'wpa-eap': 'WPA-EAP',
	'wpa2-psk': 'WPA2-PSK',
	'wpa3-psk': 'WPA3-PSK',
};

const securityLabel = computed(() => {
	const type = String(props.security_type);
	if (type === 'none') return t('components.NetworkWiFiCard.securityOpen');
	return securityLabelMap[type] || type;
});

const signalLevel = Math.min(4, Math.max(0, Math.ceil((props.signal_strength || 0) / 25)));
</script>

<template>
  <!-- La tarjeta de la librería tal cual —su superficie, su canto y su radio—:
       antes se le pisaban el fondo, el borde y el radio con `custom-class`, y
       dos clases de fondo pelean por el orden en que Tailwind las emite. La
       conectada suma un anillo del color del éxito, que no pisa nada. -->
  <ListCard
    :clickable="true"
    :custom-class="props.is_connected ? 'ring-1 ring-status-success/40' : ''"
    @click="connectToNetwork()"
  >
    <div class="flex items-center gap-3 flex-1 min-w-0">
      <div class="rounded-corner-full bg-ui-selected-accent p-2 border border-ui-line">
        <ThemeIcon :name="props.icon" type="symbol" :size="16" :alt="props.ssid" />
      </div>

      <div class="min-w-0">
        <div class="font-medium text-tx-main truncate" :title="props.ssid || props.name">{{ props.ssid || props.name }}</div>
        <div class="text-label-xs text-tx-muted flex flex-wrap items-center gap-1.5">
          <span>{{ securityLabel }}</span>
          <span v-if="props.is_connected">• {{ t('components.NetworkWiFiCard.connected') }}</span>
        </div>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div
        class="flex items-end gap-0.5"
        :title="t('components.NetworkWiFiCard.signal').replace('{0}', String(props.signal_strength || 0))"
      >
        <div
          v-for="i in 4"
          :key="i"
          class="w-1 rounded-corner-full bg-primary"
          :class="i <= signalLevel ? 'opacity-100' : 'opacity-25'"
          :style="{ height: `${4 + i * 2}px` }"
        ></div>
      </div>

      <!-- El candado es el del tema, no uno dibujado acá. -->
      <ThemeIcon
        v-if="props.security_type && String(props.security_type) !== 'none'"
        name="changes-prevent"
        type="symbol"
        :size="16"
        :alt="securityLabel"
        class="text-tx-muted"
      />

      <div
        class="w-2.5 h-2.5 rounded-corner-full"
        :class="props.is_connected ? 'bg-status-success animate-pulse' : 'bg-status-error/70'"
      ></div>
    </div>
  </ListCard>

  <!-- La contraseña se pide en el diálogo de la librería: su velo, su panel
       flotante, su foco atrapado y Escape para cerrarlo, en lugar de una caja
       fija sobre un velo negro escrito acá. -->
  <Dialog v-model:open="showModal">
    <DialogContent>
      <DialogHeader>
        <DialogTitle>
          {{ t('components.NetworkWiFiCard.connectTo').replace('{0}', String(props.ssid)) }}
        </DialogTitle>
      </DialogHeader>
      <div class="flex flex-col gap-3">
        <!-- `ariaLabel` además del marcador: un marcador no es un nombre —se
             borra al escribir, y un lector de pantalla no tiene por qué leerlo—,
             así que este campo no tenía ninguno. -->
        <TextInput
          v-model="password"
          type="password"
          autocomplete="current-password"
          :placeholder="t('components.NetworkWiFiCard.passwordPlaceholder')"
          :ariaLabel="t('components.NetworkWiFiCard.passwordPlaceholder')"
          :disabled="connecting"
        />
        <div v-if="errorMsg" class="text-status-error text-label-m">{{ errorMsg }}</div>
      </div>
      <DialogFooter>
        <ActionButton :label="t('common.cancel')" variant="secondary" @click="showModal = false" />
        <ActionButton
          :label="t('components.NetworkWiFiCard.connectAction')"
          :loading="connecting"
          :disabled="!password"
          @click="confirmConnect"
        />
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
