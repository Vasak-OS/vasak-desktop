<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import {
	connectToWifi,
	type NetworkInfo,
	type WiFiConnectionConfig,
} from '@/services/network.service';
import { getSymbolSource } from '@vasakgroup/plugin-vicons';
import { nextTick, onMounted, type Ref, ref } from 'vue';
import ActionButton from '../buttons/ActionButton.vue';
import ListCard from './ListCard.vue';

const netIcon: Ref<string> = ref('');
const props = defineProps<NetworkInfo>();

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
		// Aquí llamas al plugin con la pass
		await connectToWifi({ ssid: props.ssid, password: password.value } as WiFiConnectionConfig);
		showModal.value = false;
		password.value = '';
	} catch (error) {
		errorMsg.value = `Error al conectar: ${(error as any)?.message}`;
	} finally {
		connecting.value = false;
	}
};

onMounted(async () => {
	netIcon.value = await getSymbolSource(props.icon);
});

const securityLabelMap: Record<string, string> = {
  none: 'Abierta',
  wep: 'WEP',
  'wpa-psk': 'WPA-PSK',
  'wpa-eap': 'WPA-EAP',
  'wpa2-psk': 'WPA2-PSK',
  'wpa3-psk': 'WPA3-PSK',
};

const securityLabel = securityLabelMap[String(props.security_type)] || String(props.security_type);

const signalLevel = Math.min(4, Math.max(0, Math.ceil((props.signal_strength || 0) / 25)));
</script>

<template>
  <ListCard
    :clickable="true"
    :custom-class="`bg-ui-surface/45 border-ui-border rounded-corner px-3 py-2.5 hover:bg-ui-surface/65 transition-colors ${props.is_connected ? 'ring-1 ring-status-success/40' : ''}`"
    @click="connectToNetwork()"
  >
    <div class="flex items-center gap-3 flex-1 min-w-0">
      <div class="rounded-full bg-primary/10 p-2 border border-ui-border">
        <img :src="netIcon" :alt="props.ssid" class="w-4 h-4" />
      </div>

      <div class="min-w-0">
        <div class="font-medium text-vsk-text truncate">{{ props.ssid || props.name }}</div>
        <div class="text-xs text-vsk-text/70 flex items-center gap-1.5">
          <span>{{ securityLabel }}</span>
          <span v-if="props.is_connected">• Conectada</span>
        </div>
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div class="flex items-end gap-0.5" :title="`Señal: ${props.signal_strength || 0}%`">
        <div
          v-for="i in 4"
          :key="i"
          class="w-1 rounded-full bg-primary transition-all"
          :class="i <= signalLevel ? 'opacity-100' : 'opacity-25'"
          :style="{ height: `${4 + i * 2}px` }"
        ></div>
      </div>

      <svg
        v-if="props.security_type && String(props.security_type) !== 'none'"
        class="w-4 h-4 text-vsk-text/60"
        fill="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          d="M12,1L3,5V11C3,16.55 6.84,21.74 12,23C17.16,21.74 21,16.55 21,11V5L12,1M12,7C13.4,7 14.8,8.6 14.8,10V11H16V18H8V11H9.2V10C9.2,8.6 10.6,7 12,7M12,8.2C11.2,8.2 10.4,8.7 10.4,10V11H13.6V10C13.6,8.7 12.8,8.2 12,8.2Z"
        />
      </svg>

      <div
        class="w-2.5 h-2.5 rounded-full"
        :class="props.is_connected ? 'bg-status-success animate-pulse' : 'bg-status-danger/70'"
      ></div>
    </div>
  </ListCard>

  <!-- Modal para pedir contraseña -->
  <div v-if="showModal" class="fixed inset-0 z-50 flex items-center justify-center bg-black/45 p-4">
    <div class="w-full max-w-sm rounded-corner border border-ui-border bg-ui-bg p-4 shadow-xl flex flex-col gap-3">
      <h3 class="text-base font-semibold text-vsk-text">Conectar a {{ props.ssid }}</h3>
      <input
        v-model="password"
        type="password"
        placeholder="Contraseña WiFi"
        class="border border-ui-border rounded-corner p-2 text-vsk-text outline-none bg-ui-surface/50 focus:border-primary/40"
        :disabled="connecting"
      />
      <div v-if="errorMsg" class="text-status-danger text-sm">{{ errorMsg }}</div>
      <div class="flex gap-2 justify-end mt-2">
        <ActionButton label="Cancelar" variant="secondary" @click="showModal = false" />
        <ActionButton label="Conectar" :loading="connecting" :disabled="!password" @click="confirmConnect" />
      </div>
    </div>
  </div>
</template>
