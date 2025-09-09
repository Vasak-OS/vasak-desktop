<script lang="ts" setup>
import { NetworkInfo } from "@vasakgroup/plugin-network-manager";
import { getSymbolSource } from "@vasakgroup/plugin-vicons";
import { onMounted, Ref, ref } from "vue";

const netIcon: Ref<string> = ref("");
const props = defineProps<NetworkInfo>();

const connectToNetwork = async () => {
  if (props.is_connected) return;
  
  try {
    // Aquí llamarías al comando del plugin para conectar
    // await invoke("connect_to_network", { ssid: network.ssid });
    console.log("Connecting to:", props.ssid);
  } catch (error) {
    console.error("Error connecting to network:", error);
  }
};

onMounted(async () => {
  netIcon.value = await getSymbolSource(props.icon);
});
</script>

<template>
  <div
    @click="connectToNetwork()"
    class="flex items-center justify-between background p-3 rounded-vsk border border-vsk-primary/70 hover:bg-vsk-primary/5 cursor-pointer transition-colors"
  >
    <div class="flex items-center gap-3">
      <img :src="netIcon" :alt="props.ssid" />

      <div>
        <div class="font-medium text-vsk-text">{{ props.ssid }}</div>
        <div class="text-xs text-vsk-text/60">
          {{ props.security_type }}
          {{ props.is_connected ? "• Connected" : "" }}
        </div>
      </div>
    </div>

    <!-- Connection Status -->
    <div class="flex items-center gap-2">
      <div
        v-if="props.is_connected"
        class="w-2 h-2 rounded-full bg-green-500"
      />
      <svg
        v-if="props.security_type"
        class="w-4 h-4 text-vsk-text/60"
        fill="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          d="M12,1L3,5V11C3,16.55 6.84,21.74 12,23C17.16,21.74 21,16.55 21,11V5L12,1M12,7C13.4,7 14.8,8.6 14.8,10V11H16V18H8V11H9.2V10C9.2,8.6 10.6,7 12,7M12,8.2C11.2,8.2 10.4,8.7 10.4,10V11H13.6V10C13.6,8.7 12.8,8.2 12,8.2Z"
        />
      </svg>
    </div>
  </div>
</template>
