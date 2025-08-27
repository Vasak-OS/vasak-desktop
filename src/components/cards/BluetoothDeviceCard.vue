<script lang="ts" setup>
import { defineProps, defineEmits, Ref, ref, onMounted } from "vue";
import { getIconSource } from "@vasakgroup/plugin-vicons";
import { getDeviceInfo } from "@vasakgroup/plugin-bluetooth-manager";

const icon: Ref<string> = ref("");
const extraInfo: Ref<any> = ref({});
const props = defineProps<{
  device: any;
  actionLabel: string;
  connected?: boolean;
}>();

const emit = defineEmits(["action"]);

onMounted(async () => {
  icon.value = await getIconSource(props.device.icon || "bluetooth");
  if (props.device.path) {
    try {
      extraInfo.value = await getDeviceInfo(props.device.path);
    } catch (e) {
      extraInfo.value = {};
    }
  }
});
</script>

<template>
  <div
    class="flex items-center justify-between background rounded-vsk px-6 py-3 mb-4"
    :class="connected ? 'border-l-4 border-green-500' : ''"
  >
    <div class="flex items-center gap-3 flex-1 min-w-0">
      <img :src="icon" alt="icono" class="h-7 w-7 flex-shrink-0" />
      <div class="min-w-0">
        <div class="font-semibold truncate">
          {{ device.alias || device.name || device.address }}
        </div>
        <div class="text-xs text-gray-400 truncate">{{ device.address }}</div>
        <div
          v-if="device.icon || device.type"
          class="text-xs text-gray-400 truncate"
        >
          <span v-if="device.type" class="ml-1">{{ device.type }}</span>
        </div>
        <!-- Información adicional obtenida dinámicamente -->
        <div
          v-if="extraInfo.battery !== undefined || device.rssi || extraInfo.manufacturer !== undefined"
          class="text-xs text-gray-400 flex gap-2 mt-1"
        >
          <span v-if="extraInfo.battery !== undefined"
            >🔋 {{ extraInfo.battery }}%</span
          >
          <span v-if="device.rssi"
            >📶 {{ device.rssi }} dBm</span
          >
          <span v-if="extraInfo.manufacturer"
            >🏷️ {{ extraInfo.manufacturer }}</span
          >
        </div>
      </div>
    </div>
    <button
      class="bg-vsk-primary rounded-vsk px-4 py-2 text-sm font-semibold cursor-pointer hover:opacity-70"
      @click="$emit('action')"
    >
      {{ actionLabel }}
    </button>
  </div>
</template>

<style scoped></style>
