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
	PasswordField,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, nextTick, ref } from 'vue';
import {
	connectToWifi,
	deleteWifiConnection,
	getSavedWifiNetworks,
	type NetworkInfo,
	type WiFiConnectionConfig,
} from '@/services/network.service';

const props = defineProps<NetworkInfo>();
const { t } = useI18n();

const showModal = ref(false);
const password = ref('');
const connecting = ref(false);
const errorMsg = ref('');
const hasSavedProfile = ref(false);
const savedProfileSecurity = ref<'none' | string>('none');
const showManageSaved = ref(false); // Para gestionar perfil guardado en redes conectadas

const connectToNetwork = async () => {
	if (props.is_connected) {
		// Red ya conectada: abrir diálogo para gestionar (olvidar)
		showManageSaved.value = true;
		showModal.value = true;
		await nextTick();
		return;
	}

	// Verificar si hay perfil guardado con secrets
	try {
		const savedNetworks = await getSavedWifiNetworks();
		const saved = savedNetworks.find((n) => n.ssid === props.ssid);

		if (saved && saved.security_type !== 'none') {
			// Perfil guardado con contraseña: conectar directo
			hasSavedProfile.value = true;
			savedProfileSecurity.value = saved.security_type;
			await connectUsingSavedProfile();
		} else {
			// Sin perfil guardado o red abierta: pedir contraseña
			hasSavedProfile.value = false;
			showModal.value = true;
			await nextTick();
		}
	} catch {
		// Fallo al buscar perfiles: caer a diálogo manual
		hasSavedProfile.value = false;
		showModal.value = true;
		await nextTick();
	}
};

const connectUsingSavedProfile = async () => {
	connecting.value = true;
	errorMsg.value = '';
	try {
		// connect_to_wifi sin password usa el perfil guardado
		await connectToWifi({
			ssid: props.ssid,
			security_type: savedProfileSecurity.value as WiFiConnectionConfig['security_type'],
		} as WiFiConnectionConfig);
	} catch (error) {
		errorMsg.value = t('components.NetworkWiFiCard.connectError').replace(
			'{0}',
			String((error as any)?.message)
		);
		// Si falla, limpiar estado de perfil guardado y caer a pedir contraseña
		hasSavedProfile.value = false;
		showModal.value = true;
		await nextTick();
	} finally {
		connecting.value = false;
	}
};

const confirmConnect = async () => {
	connecting.value = true;
	errorMsg.value = '';
	try {
		await connectToWifi({
			ssid: props.ssid,
			password: password.value,
			security_type: props.security_type,
		} as WiFiConnectionConfig);
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

const forgetNetwork = async () => {
	try {
		await deleteWifiConnection(props.ssid);
		showModal.value = false;
		showManageSaved.value = false;
	} catch (error) {
		errorMsg.value = t('components.NetworkWiFiCard.connectError').replace(
			'{0}',
			String((error as any)?.message)
		);
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
  <ListCard
    :clickable="true"
    :custom-class="props.is_connected ? 'flex-wrap ring-1 ring-status-success/40' : 'flex-wrap'"
    @click="connectToNetwork()"
  >
    <div class="flex items-center gap-3 flex-1 min-w-32">
      <div class="shrink-0 rounded-corner-full bg-ui-selected-accent p-2 border border-ui-line">
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

  <Dialog v-model:open="showModal">
    <DialogContent>
      <DialogHeader>
        <DialogTitle>
          {{ t('components.NetworkWiFiCard.connectTo').replace('{0}', String(props.ssid)) }}
        </DialogTitle>
      </DialogHeader>
      <div class="flex flex-col gap-3">
        <div v-if="hasSavedProfile" class="text-tx-muted text-label-m">
          {{ t('components.NetworkWiFiCard.usingSavedProfile') }}
        </div>
        <div v-else-if="showManageSaved" class="text-tx-muted text-label-m">
          {{ t('components.NetworkWiFiCard.connectedManage') }}
        </div>
        <PasswordField
          v-if="!hasSavedProfile && !showManageSaved || props.security_type === 'none'"
          v-model="password"
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
          v-if="hasSavedProfile && props.security_type !== 'none' || showManageSaved"
          :label="t('components.NetworkWiFiCard.forgetNetwork')"
          variant="secondary"
          @click="forgetNetwork"
        />
        <ActionButton
          :label="t('components.NetworkWiFiCard.connectAction')"
          :loading="connecting"
          :disabled="!hasSavedProfile && !showManageSaved && props.security_type !== 'none' && !password"
          @click="hasSavedProfile ? connectUsingSavedProfile() : confirmConnect()"
        />
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>