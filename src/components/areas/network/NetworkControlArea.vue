<template>
  <div class="flex flex-col h-full p-4">
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
    <div class="flex items-center justify-between mb-4 p-3 rounded-vsk border border-vsk-border">
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

      <!-- Networks List -->
      <div v-else class="space-y-2 overflow-y-auto max-h-96">
        <div
          v-for="network in availableNetworks"
          :key="network.ssid"
          @click="connectToNetwork(network)"
          class="flex items-center justify-between p-3 rounded-vsk border border-vsk-border hover:bg-vsk-primary/5 cursor-pointer transition-colors"
        >
          <div class="flex items-center gap-3">
            <!-- Signal Strength Icon -->
            <div class="relative">
              <svg class="w-5 h-5 text-vsk-text" fill="currentColor" viewBox="0 0 24 24">
                <path d="M12 4C7.31 4 3.07 5.9 0 8.98L12 21L24 8.98C20.93 5.9 16.69 4 12 4M2.92 10.8C5.31 8.89 8.54 7.8 12 7.8S18.69 8.89 21.08 10.8L12 19.9L2.92 10.8Z"/>
              </svg>
              <!-- Signal bars overlay -->
              <div class="absolute inset-0 flex items-end justify-center">
                <div class="flex gap-px">
                  <div
                    v-for="i in 4"
                    :key="i"
                    :class="[
                      'w-0.5 bg-current transition-opacity',
                      i <= getSignalBars(network.signal) ? 'opacity-100' : 'opacity-30'
                    ]"
                    :style="{ height: `${i * 2 + 2}px` }"
                  />
                </div>
              </div>
            </div>
            
            <div>
              <div class="font-medium text-vsk-text">{{ network.ssid }}</div>
              <div class="text-xs text-vsk-text/60">
                {{ network.security || 'Open' }}
                {{ network.connected ? '• Connected' : '' }}
              </div>
            </div>
          </div>

          <!-- Connection Status -->
          <div class="flex items-center gap-2">
            <div
              v-if="network.connected"
              class="w-2 h-2 rounded-full bg-green-500"
            />
            <svg v-if="network.security" class="w-4 h-4 text-vsk-text/60" fill="currentColor" viewBox="0 0 24 24">
              <path d="M12,1L3,5V11C3,16.55 6.84,21.74 12,23C17.16,21.74 21,16.55 21,11V5L12,1M12,7C13.4,7 14.8,8.6 14.8,10V11H16V18H8V11H9.2V10C9.2,8.6 10.6,7 12,7M12,8.2C11.2,8.2 10.4,8.7 10.4,10V11H13.6V10C13.6,8.7 12.8,8.2 12,8.2Z"/>
            </svg>
          </div>
        </div>
      </div>

      <!-- Refresh Button -->
      <button
        @click="refreshNetworks"
        class="w-full mt-4 p-2 rounded-vsk border border-vsk-border hover:bg-vsk-primary/5 transition-colors text-sm text-vsk-text"
      >
        Refresh Networks
      </button>
    </div>

    <!-- Ethernet Section -->
    <div class="mt-6 pt-4 border-t border-vsk-border">
      <div class="flex items-center gap-3 p-3 rounded-vsk border border-vsk-border">
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
import { getCurrentNetworkState } from "@vasakgroup/plugin-network-manager"

interface Network {
  ssid: string;
  signal: number;
  security?: string;
  connected: boolean;
}

const wifiEnabled: Ref<boolean> = ref(true);
const loading: Ref<boolean> = ref(false);
const availableNetworks: Ref<Network[]> = ref([]);
const wifiStatus: Ref<string> = ref("Connected");
const ethernetStatus: Ref<string> = ref("Not connected");

const getSignalBars = (signal: number): number => {
  if (signal >= -50) return 4;
  if (signal >= -60) return 3;
  if (signal >= -70) return 2;
  return 1;
};

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
    // Aquí llamarías al comando del plugin para obtener redes
    // const networks = await invoke("get_available_networks");
    // availableNetworks.value = networks;
    
    // Mock data por ahora
    availableNetworks.value = [
      { ssid: "Home WiFi", signal: -45, security: "WPA2", connected: true },
      { ssid: "Office Network", signal: -55, security: "WPA3", connected: false },
      { ssid: "Public WiFi", signal: -65, connected: false },
      { ssid: "Neighbor's WiFi", signal: -75, security: "WPA2", connected: false },
    ];
  } catch (error) {
    console.error("Error refreshing networks:", error);
  } finally {
    loading.value = false;
  }
};

const connectToNetwork = async (network: Network) => {
  if (network.connected) return;
  
  try {
    // Aquí llamarías al comando del plugin para conectar
    // await invoke("connect_to_network", { ssid: network.ssid });
    console.log("Connecting to:", network.ssid);
  } catch (error) {
    console.error("Error connecting to network:", error);
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
  // Escuchar eventos del plugin de network
  // await listen("network-status-changed", (event) => {
  //   console.log("Network status changed:", event.payload);
  // });
  
  await refreshNetworks();
});
</script>