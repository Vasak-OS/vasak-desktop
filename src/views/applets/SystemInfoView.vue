<template>
  <div class="system-info-view">
    <h2 class="view-title">Información del Sistema</h2>
    
    <div v-if="loading" class="loading">
      <div class="spinner"></div>
      <p>Cargando información del sistema...</p>
    </div>

    <div v-else-if="error" class="error-message">
      <p>{{ error }}</p>
      <button @click="loadSystemInfo" class="retry-button">Reintentar</button>
    </div>

    <div v-else class="info-grid">
      <!-- Sistema -->
      <div class="info-card">
        <div class="card-header">
          <span class="icon">💻</span>
          <h3>Sistema Operativo</h3>
        </div>
        <div class="card-content">
          <div class="info-row">
            <span class="label">Nombre:</span>
            <span class="value">{{ systemInfo?.system.os_name }}</span>
          </div>
          <div class="info-row">
            <span class="label">Versión:</span>
            <span class="value">{{ systemInfo?.system.os_version }}</span>
          </div>
          <div class="info-row">
            <span class="label">Kernel:</span>
            <span class="value">{{ systemInfo?.system.kernel }}</span>
          </div>
          <div class="info-row">
            <span class="label">Hostname:</span>
            <span class="value">{{ systemInfo?.system.hostname }}</span>
          </div>
          <div class="info-row">
            <span class="label">Display Server:</span>
            <span class="value">{{ systemInfo?.system.display_server }}</span>
          </div>
          <div class="info-row">
            <span class="label">Uptime:</span>
            <span class="value">{{ formatUptime(systemInfo?.system.uptime_seconds) }}</span>
          </div>
        </div>
      </div>

      <!-- CPU -->
      <div class="info-card">
        <div class="card-header">
          <span class="icon">⚙️</span>
          <h3>Procesador (CPU)</h3>
        </div>
        <div class="card-content">
          <div class="info-row">
            <span class="label">Modelo:</span>
            <span class="value cpu-model">{{ systemInfo?.cpu.model }}</span>
          </div>
          <div class="info-row">
            <span class="label">Núcleos:</span>
            <span class="value">{{ systemInfo?.cpu.cores }}</span>
          </div>
          <div class="info-row" v-if="systemInfo?.cpu.frequency">
            <span class="label">Frecuencia:</span>
            <span class="value">{{ systemInfo?.cpu.frequency?.toFixed(2) }} GHz</span>
          </div>
          <div class="usage-section">
            <div class="usage-header">
              <span class="label">Uso actual:</span>
              <span class="value usage-percent" :class="usageClass(systemInfo?.cpu.usage)">
                {{ systemInfo?.cpu.usage?.toFixed(1) }}%
              </span>
            </div>
            <div class="progress-bar">
              <div 
                class="progress-fill cpu" 
                :style="{ width: systemInfo?.cpu.usage + '%' }"
                :class="usageClass(systemInfo?.cpu.usage)"
              ></div>
            </div>
          </div>
        </div>
      </div>

      <!-- Memoria -->
      <div class="info-card">
        <div class="card-header">
          <span class="icon">🧠</span>
          <h3>Memoria RAM</h3>
        </div>
        <div class="card-content">
          <div class="info-row">
            <span class="label">Total:</span>
            <span class="value">{{ systemInfo?.memory.total_gb?.toFixed(2) }} GB</span>
          </div>
          <div class="info-row">
            <span class="label">En uso:</span>
            <span class="value">{{ systemInfo?.memory.used_gb?.toFixed(2) }} GB</span>
          </div>
          <div class="info-row">
            <span class="label">Disponible:</span>
            <span class="value">{{ systemInfo?.memory.available_gb?.toFixed(2) }} GB</span>
          </div>
          <div class="usage-section">
            <div class="usage-header">
              <span class="label">Uso:</span>
              <span class="value usage-percent" :class="usageClass(systemInfo?.memory.usage_percent)">
                {{ systemInfo?.memory.usage_percent?.toFixed(1) }}%
              </span>
            </div>
            <div class="progress-bar">
              <div 
                class="progress-fill memory" 
                :style="{ width: systemInfo?.memory.usage_percent + '%' }"
                :class="usageClass(systemInfo?.memory.usage_percent)"
              ></div>
            </div>
          </div>
        </div>
      </div>

      <!-- GPU -->
      <div class="info-card" v-if="systemInfo?.gpu">
        <div class="card-header">
          <span class="icon">🎮</span>
          <h3>Tarjeta Gráfica (GPU)</h3>
        </div>
        <div class="card-content">
          <div class="info-row">
            <span class="label">Fabricante:</span>
            <span class="value">{{ systemInfo?.gpu.vendor }}</span>
          </div>
          <div class="info-row">
            <span class="label">Modelo:</span>
            <span class="value gpu-model">{{ systemInfo?.gpu.model }}</span>
          </div>
        </div>
      </div>

      <!-- Temperatura -->
      <div class="info-card" v-if="systemInfo?.temperature">
        <div class="card-header">
          <span class="icon">🌡️</span>
          <h3>Temperatura</h3>
        </div>
        <div class="card-content">
          <div class="info-row" v-if="systemInfo?.temperature.cpu_temp">
            <span class="label">CPU:</span>
            <span class="value temp" :class="tempClass(systemInfo?.temperature.cpu_temp)">
              {{ systemInfo?.temperature.cpu_temp?.toFixed(1) }}°C
            </span>
          </div>
          <div class="sensors-list" v-if="systemInfo?.temperature.sensors?.length">
            <div class="sensor-item" v-for="sensor in systemInfo.temperature.sensors.slice(0, 3)" :key="sensor.name">
              <span class="sensor-label">{{ sensor.label }}:</span>
              <span class="sensor-temp" :class="tempClass(sensor.temp)">{{ sensor.temp.toFixed(1) }}°C</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="actions" v-if="!loading && !error">
      <button @click="() => loadSystemInfo()" class="refresh-button" :disabled="isUpdating">
        <span class="refresh-icon" :class="{ 'spinning': isUpdating }">🔄</span>
        {{ isUpdating ? 'Actualizando...' : 'Actualizar' }}
      </button>
      <p class="update-info">
        <span v-if="isUpdating" class="updating-indicator">● Actualizando...</span>
        <span v-else>Última actualización: {{ lastUpdate }}</span>
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface SystemInfo {
  cpu: {
    model: string
    cores: number
    usage: number
    frequency?: number
  }
  memory: {
    total_gb: number
    used_gb: number
    available_gb: number
    usage_percent: number
  }
  gpu?: {
    model: string
    vendor: string
  }
  system: {
    hostname: string
    kernel: string
    os_name: string
    os_version: string
    display_server: string
    uptime_seconds: number
  }
  temperature?: {
    cpu_temp?: number
    sensors: Array<{
      name: string
      temp: number
      label: string
    }>
  }
}

const systemInfo = ref<SystemInfo | null>(null)
const loading = ref(true)
const isInitialLoad = ref(true)
const isUpdating = ref(false)
const error = ref('')
const lastUpdate = ref('')
let updateInterval: number | null = null

const loadSystemInfo = async (silent = false) => {
  try {
    // Solo mostrar loading completo en la carga inicial
    if (!silent) {
      loading.value = true
    } else {
      isUpdating.value = true
    }
    error.value = ''
    systemInfo.value = await invoke<SystemInfo>('get_system_info')
    lastUpdate.value = new Date().toLocaleTimeString()
    isInitialLoad.value = false
  } catch (e) {
    error.value = 'Error al cargar información del sistema: ' + e
    console.error(e)
  } finally {
    loading.value = false
    isUpdating.value = false
  }
}

const formatUptime = (seconds: number | undefined): string => {
  if (!seconds) return '0 segundos'
  
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  
  const parts = []
  if (days > 0) parts.push(`${days}d`)
  if (hours > 0) parts.push(`${hours}h`)
  if (minutes > 0) parts.push(`${minutes}m`)
  
  return parts.join(' ') || '< 1m'
}

const usageClass = (usage: number | undefined): string => {
  if (!usage) return ''
  if (usage > 80) return 'high'
  if (usage > 60) return 'medium'
  return 'low'
}

const tempClass = (temp: number | undefined): string => {
  if (!temp) return ''
  if (temp > 80) return 'critical'
  if (temp > 60) return 'warm'
  return 'normal'
}

onMounted(() => {
  loadSystemInfo()
  // Actualizar cada 5 segundos en modo silencioso (sin loading)
  updateInterval = setInterval(() => loadSystemInfo(true), 5000) as unknown as number
})

onUnmounted(() => {
  if (updateInterval) {
    clearInterval(updateInterval)
  }
})
</script>

<style scoped>
.system-info-view {
  padding: 2rem;
  max-width: 1200px;
  margin: 0 auto;
}

.view-title {
  font-size: 2rem;
  font-weight: 700;
  margin-bottom: 2rem;
  color: var(--text-primary);
}

.loading, .error-message {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem;
  text-align: center;
}

.spinner {
  width: 48px;
  height: 48px;
  border: 4px solid var(--surface-3);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
  gap: 1.5rem;
  margin-bottom: 2rem;
}

.info-card {
  background: var(--surface-2);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  transition: transform 0.2s, box-shadow 0.2s;
}

.info-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.card-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem 1.25rem;
  background: var(--surface-3);
  border-bottom: 1px solid var(--border);
}

.card-header .icon {
  font-size: 1.5rem;
}

.card-header h3 {
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.card-content {
  padding: 1.25rem;
}

.info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 0;
  border-bottom: 1px solid var(--border-subtle);
}

.info-row:last-child {
  border-bottom: none;
}

.label {
  font-weight: 500;
  color: var(--text-secondary);
  font-size: 0.9rem;
}

.value {
  font-weight: 600;
  color: var(--text-primary);
  text-align: right;
  max-width: 60%;
  word-break: break-word;
}

.cpu-model, .gpu-model {
  font-size: 0.85rem;
  line-height: 1.3;
}

.usage-section {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border);
}

.usage-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 0.5rem;
}

.usage-percent {
  font-size: 1.1rem;
  font-weight: 700;
}

.usage-percent.low { color: #4ade80; }
.usage-percent.medium { color: #fbbf24; }
.usage-percent.high { color: #f87171; }

.progress-bar {
  height: 8px;
  background: var(--surface-1);
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  transition: width 0.3s ease;
  border-radius: 4px;
}

.progress-fill.cpu.low,
.progress-fill.memory.low {
  background: linear-gradient(90deg, #4ade80, #22c55e);
}

.progress-fill.cpu.medium,
.progress-fill.memory.medium {
  background: linear-gradient(90deg, #fbbf24, #f59e0b);
}

.progress-fill.cpu.high,
.progress-fill.memory.high {
  background: linear-gradient(90deg, #f87171, #ef4444);
}

.temp {
  font-weight: 700;
}

.temp.normal { color: #4ade80; }
.temp.warm { color: #fbbf24; }
.temp.critical { color: #f87171; }

.sensors-list {
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--border-subtle);
}

.sensor-item {
  display: flex;
  justify-content: space-between;
  padding: 0.35rem 0;
  font-size: 0.9rem;
}

.sensor-label {
  color: var(--text-secondary);
}

.sensor-temp {
  font-weight: 600;
}

.actions {
  text-align: center;
  padding: 1.5rem;
  background: var(--surface-2);
  border-radius: 12px;
}

.refresh-button, .retry-button {
  padding: 0.75rem 1.5rem;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 8px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s, transform 0.1s;
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
}

.refresh-button:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.refresh-button:hover:not(:disabled), .retry-button:hover {
  background: var(--accent-hover);
  transform: scale(1.02);
}

.refresh-button:active:not(:disabled), .retry-button:active {
  transform: scale(0.98);
}

.refresh-icon {
  display: inline-block;
  transition: transform 0.3s ease;
}

.refresh-icon.spinning {
  animation: spin 1s linear infinite;
}

.update-info {
  margin-top: 0.75rem;
  font-size: 0.85rem;
  color: var(--text-tertiary);
}

.updating-indicator {
  color: var(--accent);
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.error-message {
  color: var(--error);
}

@media (max-width: 768px) {
  .info-grid {
    grid-template-columns: 1fr;
  }
  
  .system-info-view {
    padding: 1rem;
  }
}
</style>
