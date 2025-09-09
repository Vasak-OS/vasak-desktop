<template>
  <div class="flex flex-col h-full p-2">
    <!-- Header -->
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-semibold text-vsk-text">Network Settings</h2>
      <button
        @click="closeApplet"
        class="p-2 rounded-vsk hover:bg-vsk-primary/10 transition-colors"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
        </svg>
      </button>
    </div>

    <!-- WiFi Toggle -->
    <div class="flex items-center justify-between mb-4 p-3 rounded-vsk border border-vsk-primary/70 background">
      <div class="flex items-center gap-3">
        <div class="p-2 rounded-full bg-vsk-primary/10">
          <svg class="w-5 h-5 text-vsk-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" 
              d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0"></path>
          </svg>
        </div>
        <div>
          <h3 class="font-medium text-vsk-text">WiFi</h3>
          <p class="text-sm text-vsk-text/70">{{ wifiStatus }}</p>
        </div>
      </div>
      <button
        @click="toggleWifi"
        disabled
        :class="[
          'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
          wifiEnabled ? 'bg-vsk-primary' : 'bg-vsk-border'
        ]"
      >
        <span
          :class="[
            'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
            wifiEnabled ? 'translate-x-6' : 'translate-x-1'
          ]"
        />
      </button>
    </div>

    <!-- Available Networks -->
    <div v-if="wifiEnabled" class="flex-1 overflow-hidden">
      <h3 class="text-sm font-medium text-vsk-text/80 mb-3">Available Networks</h3>
      
      <!-- Loading -->
      <div v-if="loading" class="flex items-center justify-center py-8">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-vsk-primary"></div>
      </div>

      <div v-else class="space-y-2 overflow-y-auto max-h-96">
        <NetworkWiFiCard v-for="network in availableNetworks" :key="network.ssid" v-bind="network" />
      </div>

      <!-- Refresh Button -->
      <button
        @click="refreshNetworks"
        class="w-full mt-4 p-2 rounded-vsk border border-vsk-primary/70 hover:bg-vsk-primary/5 transition-colors text-sm text-vsk-text"
      >
        Refresh Networks
      </button>
    </div>

    <!-- Ethernet Section -->
    <div class="mt-6 pt-4 border-t border-vsk-primary/70">
      <div class="flex items-center gap-3 p-3 background rounded-vsk border border-vsk-primary/70">
        <div class="p-2 rounded-full bg-vsk-primary/10">
          <svg class="w-5 h-5 text-vsk-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" 
              d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2"></path>
          </svg>
        </div>
        <div>
          <h3 class="font-medium text-vsk-text">Ethernet</h3>
          <p class="text-sm text-vsk-text/70">{{ ethernetStatus }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listWifiNetworks, NetworkInfo } from "@vasakgroup/plugin-network-manager"
import NetworkWiFiCard from "@/components/cards/NetworkWiFiCard.vue";

const wifiEnabled: Ref<boolean> = ref(true);
const loading: Ref<boolean> = ref(false);
const availableNetworks: Ref<NetworkInfo[]> = ref([]);
const wifiStatus: Ref<string> = ref("Connected");
const ethernetStatus: Ref<string> = ref("Not connected");

const toggleWifi = async () => {
  try {
    // Aquí llamarías al comando del plugin para toggle WiFi
    // await invoke("toggle_wifi");
    wifiEnabled.value = !wifiEnabled.value;
    if (wifiEnabled.value) {
      await refreshNetworks();
    }
  } catch (error) {
    console.error("Error toggling WiFi:", error);
  }
};

const refreshNetworks = async () => {
  if (!wifiEnabled.value) return;
  
  loading.value = true;
  try {
    availableNetworks.value = await listWifiNetworks();
  } catch (error) {
    console.error("Error refreshing networks:", error);
  } finally {
    loading.value = false;
  }
};

const closeApplet = async () => {
  try {
    await invoke("toggle_network_applet");
  } catch (error) {
    console.error("Error closing applet:", error);
  }
};

onMounted(async () => {
  await refreshNetworks();
});
</script>