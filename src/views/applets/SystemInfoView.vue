<template>
  <div class="p-8 max-w-[1200px] mx-auto">
    <h2 class="text-3xl font-bold mb-8 text-vsk-primary">
      Información del Sistema
    </h2>

    <div
      v-if="loading"
      class="flex flex-col items-center justify-center p-12 text-center"
    >
      <div class="w-12 h-12 border-4 rounded-full animate-spin"></div>
      <p>Cargando información del sistema...</p>
    </div>

    <div
      v-else-if="error"
      class="flex flex-col items-center justify-center p-12 text-center text-red-500"
    >
      <p>{{ error }}</p>
      <button
        @click="() => loadSystemInfo()"
        class="mt-3 py-3 px-6 bg-vsk-primary border-0 rounded-vsk font-semibold cursor-pointer transition-all hover:bg-[var(--accent-hover)] hover:scale-105 active:scale-95"
      >
        Reintentar
      </button>
    </div>

    <div
      v-else
      class="grid grid-cols-[repeat(auto-fit,minmax(350px,1fr))] gap-6 mb-8"
    >
      <!-- Sistema -->
      <div
        class="background rounded-vsk overflow-hidden shadow-[0_2px_8px_rgba(0,0,0,0.1)] transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_12px_rgba(0,0,0,0.15)]"
      >
        <div class="flex items-center gap-3 py-4 px-5">
          <span class="text-2xl">💻</span>
          <h3 class="text-lg font-semibold m-0">Sistema Operativo</h3>
        </div>
        <div class="p-5">
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">Nombre:</span>
            <span class="font-semibold text-right max-w-[60%]">{{
              systemInfo?.system.os_name
            }}</span>
          </div>
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">Versión:</span>
            <span class="font-semibold text-right max-w-[60%]">{{
              systemInfo?.system.os_version
            }}</span>
          </div>
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">Kernel:</span>
            <span class="font-semibold text-right max-w-[60%]">{{
              systemInfo?.system.kernel
            }}</span>
          </div>
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">Hostname:</span>
            <span class="font-semibold text-right max-w-[60%]">{{
              systemInfo?.system.hostname
            }}</span>
          </div>
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm"
              >Display Server:</span
            >
            <span class="font-semibold text-right max-w-[60%]">{{
              systemInfo?.system.display_server
            }}</span>
          </div>
          <div class="flex justify-between items-center py-2 border-b-0">
            <span class="font-medium text-vsk-primary text-sm">Uptime:</span>
            <span class="font-semibold text-right max-w-[60%]">{{
              formatUptime(systemInfo?.system.uptime_seconds)
            }}</span>
          </div>
        </div>
      </div>

      <!-- CPU -->
      <div
        class="background rounded-vsk overflow-hidden shadow-[0_2px_8px_rgba(0,0,0,0.1)] transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_12px_rgba(0,0,0,0.15)]"
      >
        <div class="flex items-center gap-3 py-4 px-5">
          <span class="text-2xl">⚙️</span>
          <h3 class="text-lg font-semibold m-0">Procesador (CPU)</h3>
        </div>
        <div class="p-5">
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">Modelo:</span>
            <span
              class="font-semibold text-right max-w-[60%] text-[0.85rem] leading-tight"
              >{{ systemInfo?.cpu.model }}</span
            >
          </div>
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">Núcleos:</span>
            <span class="font-semibold text-right max-w-[60%]">{{
              systemInfo?.cpu.cores
            }}</span>
          </div>
          <div
            class="flex justify-between items-center py-2"
            v-if="systemInfo?.cpu.frequency"
          >
            <span class="font-medium text-vsk-primary text-sm"
              >Frecuencia:</span
            >
            <span class="font-semibold text-right max-w-[60%]"
              >{{ systemInfo?.cpu.frequency?.toFixed(2) }} GHz</span
            >
          </div>
          <div class="mt-4 pt-4 border-t border-[var(--border)]">
            <div class="flex justify-between mb-2">
              <span class="font-medium text-vsk-primary text-sm"
                >Uso actual:</span
              >
              <span
                class="font-bold text-lg"
                :class="{
                  'text-[#4ade80]': usageClass(systemInfo?.cpu.usage) === 'low',
                  'text-[#fbbf24]':
                    usageClass(systemInfo?.cpu.usage) === 'medium',
                  'text-[#f87171]':
                    usageClass(systemInfo?.cpu.usage) === 'high',
                }"
              >
                {{ systemInfo?.cpu.usage?.toFixed(1) }}%
              </span>
            </div>
            <div class="h-2 bg-[var(--surface-1)] rounded overflow-hidden">
              <div
                class="h-full transition-all duration-300 rounded"
                :style="{ width: systemInfo?.cpu.usage + '%' }"
                :class="{
                  'bg-gradient-to-r from-[#4ade80] to-[#22c55e]':
                    usageClass(systemInfo?.cpu.usage) === 'low',
                  'bg-gradient-to-r from-[#fbbf24] to-[#f59e0b]':
                    usageClass(systemInfo?.cpu.usage) === 'medium',
                  'bg-gradient-to-r from-[#f87171] to-[#ef4444]':
                    usageClass(systemInfo?.cpu.usage) === 'high',
                }"
              ></div>
            </div>
          </div>
        </div>
      </div>

      <!-- Memoria -->
      <div
        class="background rounded-vsk overflow-hidden shadow-[0_2px_8px_rgba(0,0,0,0.1)] transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_12px_rgba(0,0,0,0.15)]"
      >
        <div class="flex items-center gap-3 py-4 px-5">
          <span class="text-2xl">🧠</span>
          <h3 class="text-lg font-semibold m-0">Memoria RAM</h3>
        </div>
        <div class="p-5">
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">Total:</span>
            <span class="font-semibold text-right max-w-[60%]"
              >{{ systemInfo?.memory.total_gb?.toFixed(2) }} GB</span
            >
          </div>
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">En uso:</span>
            <span class="font-semibold text-right max-w-[60%]"
              >{{ systemInfo?.memory.used_gb?.toFixed(2) }} GB</span
            >
          </div>
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm"
              >Disponible:</span
            >
            <span class="font-semibold text-right max-w-[60%]"
              >{{ systemInfo?.memory.available_gb?.toFixed(2) }} GB</span
            >
          </div>
          <div class="mt-4 pt-4 border-t border-[var(--border)]">
            <div class="flex justify-between mb-2">
              <span class="font-medium text-vsk-primary text-sm">Uso:</span>
              <span
                class="font-bold text-lg"
                :class="{
                  'text-[#4ade80]':
                    usageClass(systemInfo?.memory.usage_percent) === 'low',
                  'text-[#fbbf24]':
                    usageClass(systemInfo?.memory.usage_percent) === 'medium',
                  'text-[#f87171]':
                    usageClass(systemInfo?.memory.usage_percent) === 'high',
                }"
              >
                {{ systemInfo?.memory.usage_percent?.toFixed(1) }}%
              </span>
            </div>
            <div class="h-2 bg-[var(--surface-1)] rounded overflow-hidden">
              <div
                class="h-full transition-all duration-300 rounded"
                :style="{ width: systemInfo?.memory.usage_percent + '%' }"
                :class="{
                  'bg-gradient-to-r from-[#4ade80] to-[#22c55e]':
                    usageClass(systemInfo?.memory.usage_percent) === 'low',
                  'bg-gradient-to-r from-[#fbbf24] to-[#f59e0b]':
                    usageClass(systemInfo?.memory.usage_percent) === 'medium',
                  'bg-gradient-to-r from-[#f87171] to-[#ef4444]':
                    usageClass(systemInfo?.memory.usage_percent) === 'high',
                }"
              ></div>
            </div>
          </div>
        </div>
      </div>

      <!-- Swap -->
      <div
        class="background rounded-vsk overflow-hidden shadow-[0_2px_8px_rgba(0,0,0,0.1)] transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_12px_rgba(0,0,0,0.15)]"
        v-if="systemInfo?.swap"
      >
        <div class="flex items-center gap-3 py-4 px-5">
          <span class="text-2xl">🔁</span>
          <h3 class="text-lg font-semibold m-0">Swap</h3>
        </div>
        <div class="p-5">
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">Total:</span>
            <span class="font-semibold text-right max-w-[60%]"
              >{{ systemInfo?.swap?.total_gb?.toFixed(2) }} GB</span
            >
          </div>
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">En uso:</span>
            <span class="font-semibold text-right max-w-[60%]"
              >{{ systemInfo?.swap?.used_gb?.toFixed(2) }} GB</span
            >
          </div>
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm">Libre:</span>
            <span class="font-semibold text-right max-w-[60%]"
              >{{ systemInfo?.swap?.free_gb?.toFixed(2) }} GB</span
            >
          </div>
          <div class="mt-4 pt-4 border-t border-[var(--border)]">
            <div class="flex justify-between mb-2">
              <span class="font-medium text-vsk-primary text-sm">Uso:</span>
              <span
                class="font-bold text-lg"
                :class="{
                  'text-[#4ade80]':
                    usageClass(systemInfo?.swap?.usage_percent) === 'low',
                  'text-[#fbbf24]':
                    usageClass(systemInfo?.swap?.usage_percent) === 'medium',
                  'text-[#f87171]':
                    usageClass(systemInfo?.swap?.usage_percent) === 'high',
                }"
              >
                {{ systemInfo?.swap?.usage_percent?.toFixed(1) }}%
              </span>
            </div>
            <div class="h-2 bg-[var(--surface-1)] rounded overflow-hidden">
              <div
                class="h-full transition-all duration-300 rounded"
                :style="{ width: systemInfo?.swap?.usage_percent + '%' }"
                :class="{
                  'bg-gradient-to-r from-[#4ade80] to-[#22c55e]':
                    usageClass(systemInfo?.swap?.usage_percent) === 'low',
                  'bg-gradient-to-r from-[#fbbf24] to-[#f59e0b]':
                    usageClass(systemInfo?.swap?.usage_percent) === 'medium',
                  'bg-gradient-to-r from-[#f87171] to-[#ef4444]':
                    usageClass(systemInfo?.swap?.usage_percent) === 'high',
                }"
              ></div>
            </div>
          </div>
        </div>
      </div>

      <!-- GPU -->
      <div
        class="background rounded-vsk overflow-hidden shadow-[0_2px_8px_rgba(0,0,0,0.1)] transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_12px_rgba(0,0,0,0.15)]"
        v-if="systemInfo?.gpu"
      >
        <div class="flex items-center gap-3 py-4 px-5">
          <span class="text-2xl">🎮</span>
          <h3 class="text-lg font-semibold m-0">Tarjeta Gráfica (GPU)</h3>
        </div>
        <div class="p-5">
          <div class="flex justify-between items-center py-2">
            <span class="font-medium text-vsk-primary text-sm"
              >Fabricante:</span
            >
            <span class="font-semibold text-right max-w-[60%]">{{
              systemInfo?.gpu.vendor
            }}</span>
          </div>
          <div class="flex justify-between items-center py-2 border-b-0">
            <span class="font-medium text-vsk-primary text-sm">Modelo:</span>
            <span
              class="font-semibold text-right max-w-[60%] text-[0.85rem] leading-tight"
              >{{ systemInfo?.gpu.model }}</span
            >
          </div>
        </div>
      </div>

      <!-- Temperatura -->
      <div
        class="background rounded-vsk overflow-hidden shadow-[0_2px_8px_rgba(0,0,0,0.1)] transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_12px_rgba(0,0,0,0.15)]"
        v-if="systemInfo?.temperature"
      >
        <div class="flex items-center gap-3 py-4 px-5">
          <span class="text-2xl">🌡️</span>
          <h3 class="text-lg font-semibold m-0">Temperatura</h3>
        </div>
        <div class="p-5">
          <div
            class="flex justify-between items-center py-2"
            v-if="systemInfo?.temperature.cpu_temp"
          >
            <span class="font-medium text-vsk-primary text-sm">CPU:</span>
            <span
              class="font-bold text-right max-w-[60%]"
              :class="{
                'text-[#4ade80]':
                  tempClass(systemInfo?.temperature.cpu_temp) === 'normal',
                'text-[#fbbf24]':
                  tempClass(systemInfo?.temperature.cpu_temp) === 'warm',
                'text-[#f87171]':
                  tempClass(systemInfo?.temperature.cpu_temp) === 'critical',
              }"
            >
              {{ systemInfo?.temperature.cpu_temp?.toFixed(1) }}°C
            </span>
          </div>
          <div
            class="mt-3 pt-3 border-t border-[var(--border-subtle)]"
            v-if="systemInfo?.temperature.sensors?.length"
          >
            <div
              class="flex justify-between py-1.5 text-sm"
              v-for="sensor in systemInfo.temperature.sensors.slice(0, 3)"
              :key="sensor.name"
            >
              <span class="text-vsk-primary">{{ sensor.label }}:</span>
              <span
                class="font-semibold"
                :class="{
                  'text-[#4ade80]': tempClass(sensor.temp) === 'normal',
                  'text-[#fbbf24]': tempClass(sensor.temp) === 'warm',
                  'text-[#f87171]': tempClass(sensor.temp) === 'critical',
                }"
                >{{ sensor.temp.toFixed(1) }}°C</span
              >
            </div>
          </div>
        </div>
      </div>

      <!-- Discos -->
      <div
        class="background rounded-vsk overflow-hidden shadow-[0_2px_8px_rgba(0,0,0,0.1)] transition-all hover:-translate-y-0.5 hover:shadow-[0_4px_12px_rgba(0,0,0,0.15)]"
        v-if="systemInfo?.disks?.length"
      >
        <div class="flex items-center gap-3 py-4 px-5">
          <span class="text-2xl">💽</span>
          <h3 class="text-lg font-semibold m-0">Discos</h3>
        </div>
        <div class="p-5">
          <div
            class="py-2 pb-4 last:border-b-0"
            v-for="disk in systemInfo?.disks"
            :key="disk.device + disk.mountpoint"
          >
            <div
              class="flex flex-wrap gap-2 items-center mb-2 text-vsk-primary"
            >
              <span class="font-semibold">{{ disk.device }}</span>
              <span class="text-sm">→ {{ disk.mountpoint }}</span>
              <span class="text-[0.85rem]">({{ disk.fstype }})</span>
            </div>
            <div class="flex flex-wrap gap-4 mb-2">
              <span>Total: {{ disk.total_gb.toFixed(2) }} GB</span>
              <span>Usado: {{ disk.used_gb.toFixed(2) }} GB</span>
              <span>Libre: {{ disk.available_gb.toFixed(2) }} GB</span>
              <span
                class="font-bold text-lg"
                :class="{
                  'text-[#4ade80]': usageClass(disk.usage_percent) === 'low',
                  'text-[#fbbf24]': usageClass(disk.usage_percent) === 'medium',
                  'text-[#f87171]': usageClass(disk.usage_percent) === 'high',
                }"
              >
                {{ disk.usage_percent.toFixed(1) }}%
              </span>
            </div>
            <div class="h-2 bg-[var(--surface-1)] rounded overflow-hidden">
              <div
                class="h-full transition-all duration-300 rounded"
                :style="{ width: disk.usage_percent + '%' }"
                :class="{
                  'bg-gradient-to-r from-[#4ade80] to-[#22c55e]':
                    usageClass(disk.usage_percent) === 'low',
                  'bg-gradient-to-r from-[#fbbf24] to-[#f59e0b]':
                    usageClass(disk.usage_percent) === 'medium',
                  'bg-gradient-to-r from-[#f87171] to-[#ef4444]':
                    usageClass(disk.usage_percent) === 'high',
                }"
              ></div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div
      class="text-center p-6 background rounded-vsk"
      v-if="!loading && !error"
    >
      <button
        @click="() => loadSystemInfo()"
        class="py-3 px-6 bg-vsk-primary text-white border-0 rounded-vsk font-semibold cursor-pointer transition-all inline-flex items-center gap-2 hover:bg-[var(--accent-hover)] hover:scale-105 active:scale-95 disabled:opacity-70 disabled:cursor-not-allowed"
        :disabled="isUpdating"
      >
        <span
          class="inline-block transition-transform duration-300"
          :class="{ 'animate-spin': isUpdating }"
          >🔄</span
        >
        {{ isUpdating ? "Actualizando..." : "Actualizar" }}
      </button>
      <p class="mt-3 text-[0.85rem] text-[var(--text-tertiary)]">
        <span v-if="isUpdating" class="text-vsk-primary animate-pulse"
          >● Actualizando...</span
        >
        <span v-else>Última actualización: {{ lastUpdate }}</span>
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMounted, onUnmounted, ref } from 'vue';
import { logError } from '@/utils/logger';

interface SystemInfo {
	cpu: {
		model: string;
		cores: number;
		usage: number;
		frequency?: number;
	};
	memory: {
		total_gb: number;
		used_gb: number;
		available_gb: number;
		usage_percent: number;
	};
	swap?: {
		total_gb: number;
		used_gb: number;
		free_gb: number;
		usage_percent: number;
	};
	gpu?: {
		model: string;
		vendor: string;
	};
	system: {
		hostname: string;
		kernel: string;
		os_name: string;
		os_version: string;
		display_server: string;
		uptime_seconds: number;
	};
	temperature?: {
		cpu_temp?: number;
		sensors: Array<{
			name: string;
			temp: number;
			label: string;
		}>;
	};
	disks?: Array<{
		device: string;
		mountpoint: string;
		fstype: string;
		total_gb: number;
		used_gb: number;
		available_gb: number;
		usage_percent: number;
	}>;
}

const systemInfo = ref<SystemInfo | null>(null);
const loading = ref(true);
const isInitialLoad = ref(true);
const isUpdating = ref(false);
const error = ref('');
const lastUpdate = ref('');
let updateInterval: number | null = null;

const loadSystemInfo = async (silent = false) => {
	try {
		if (silent) {
			isUpdating.value = true;
		} else {
			loading.value = true;
		}
		error.value = '';
		systemInfo.value = await invoke<SystemInfo>('get_system_info');
		lastUpdate.value = new Date().toLocaleTimeString();
		isInitialLoad.value = false;
	} catch (e) {
		error.value = 'Error al cargar información del sistema: ' + e;
		console.error(e);
	} finally {
		loading.value = false;
		isUpdating.value = false;
	}
};

const formatUptime = (seconds: number | undefined): string => {
	if (!seconds) return '0 segundos';

	const days = Math.floor(seconds / 86400);
	const hours = Math.floor((seconds % 86400) / 3600);
	const minutes = Math.floor((seconds % 3600) / 60);

	const parts = [];
	if (days > 0) parts.push(`${days}d`);
	if (hours > 0) parts.push(`${hours}h`);
	if (minutes > 0) parts.push(`${minutes}m`);

	return parts.join(' ') || '< 1m';
};

const usageClass = (usage: number | undefined): string => {
	if (!usage) return '';
	if (usage > 80) return 'high';
	if (usage > 60) return 'medium';
	return 'low';
};

const tempClass = (temp: number | undefined): string => {
	if (!temp) return '';
	if (temp > 80) return 'critical';
	if (temp > 60) return 'warm';
	return 'normal';
};

onMounted(() => {
	loadSystemInfo();
	// Actualizar cada 5 segundos en modo silencioso (sin loading)
	updateInterval = setInterval(() => loadSystemInfo(true), 5000) as unknown as number;
});

onUnmounted(() => {
	if (updateInterval) {
		clearInterval(updateInterval);
	}
});
</script>
