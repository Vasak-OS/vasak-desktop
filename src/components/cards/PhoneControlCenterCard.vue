<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, type Ref, ref } from 'vue';
import type { ConnectDevice, ConnectRunningApp } from '@/interfaces/connect';
import {
	listConnectDevices,
	listConnectRunning,
	stopConnectApp,
	toggleConnectMenu,
} from '@/services/connect.service';
import { useIcons } from '@/tools/composables/useReactiveIcon';
import { useSharedEvent } from '@/tools/event.bus';

/**
 * The phone's state, in the notification centre.
 *
 * Renders nothing when there is no phone. A card explaining that a feature is
 * unavailable, permanently, in the panel people open to read notifications, is
 * worse than no card.
 */
const { t } = useI18n();

const devices: Ref<ConnectDevice[]> = ref([]);
const running: Ref<ConnectRunningApp[]> = ref([]);

const { phoneIcon } = useIcons({ phoneIcon: 'smartphone' });

const device = computed(() => devices.value[0]);

const refresh = async () => {
	devices.value = await listConnectDevices();
	running.value = devices.value.length > 0 ? await listConnectRunning() : [];
};

const close = async (app: ConnectRunningApp) => {
	await stopConnectApp(app.serial, app.package);
	running.value = await listConnectRunning();
};

onMounted(refresh);

useSharedEvent('connect-device-added', refresh);
useSharedEvent('connect-device-changed', refresh);
useSharedEvent('connect-device-removed', refresh);
useSharedEvent('connect-app-closed', refresh);
</script>

<template>
  <div v-if="device" class="flex flex-col gap-2 rounded-corner bg-ui-surface/40 p-3 text-tx-main">
    <button type="button" class="flex items-center gap-3 text-left" @click="toggleConnectMenu()">
      <img :src="phoneIcon" alt="" class="h-8 w-8 shrink-0" />
      <div class="min-w-0 flex-1">
        <p class="truncate font-semibold text-tx-main">{{ device.model }}</p>
        <p class="truncate text-xs text-tx-muted">
          <span v-if="device.state === 'unauthorized'" class="text-status-warning">
            {{ t('views.connect.unauthorized') }}
          </span>
          <span v-else-if="device.state === 'ready'">
            {{ device.transport === 'usb' ? 'USB' : device.address }}
            <template v-if="running.length > 0">
              · {{ t('views.connect.openApps').replace('{0}', String(running.length)) }}
            </template>
          </span>
          <span v-else>{{ t('views.connect.connecting') }}</span>
        </p>
      </div>
      <div
        class="h-2.5 w-2.5 shrink-0 rounded-full"
        :class="{
          'bg-status-success': device.state === 'ready',
          'bg-status-warning': device.state === 'unauthorized',
          'bg-tx-muted': device.state !== 'ready' && device.state !== 'unauthorized',
        }"
      ></div>
    </button>

    <!-- The open windows, with a way to close them. A window whose app is on a
         virtual display is easy to lose behind others, and this is the only
         place that knows they exist. -->
    <ul v-if="running.length > 0" class="space-y-1">
      <li
        v-for="app in running"
        :key="app.package"
        class="flex items-center gap-2 rounded-corner px-2 py-1 text-sm hover:bg-primary/10"
      >
        <span class="min-w-0 flex-1 truncate text-tx-main">{{ app.label }}</span>
        <button
          type="button"
          :title="t('views.connect.close')"
          class="shrink-0 rounded-corner px-2 text-xs text-primary hover:bg-primary hover:text-tx-on-primary"
          @click="close(app)"
        >
          {{ t('views.connect.close') }}
        </button>
      </li>
    </ul>
  </div>
</template>
