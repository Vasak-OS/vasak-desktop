<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onBeforeUnmount, onMounted, type Ref, ref } from 'vue';
import type { ConnectApp, ConnectDevice, ConnectRunningApp } from '@/interfaces/connect';
import {
	launchConnectApp,
	listConnectApps,
	listConnectDevices,
	listConnectRunning,
	stopConnectApp,
} from '@/services/connect.service';
import { useIcons } from '@/tools/composables/useReactiveIcon';
import { logError } from '@/utils/logger';

const { t } = useI18n();

const menuWindow = getCurrentWindow();
const devices: Ref<ConnectDevice[]> = ref([]);
const selected: Ref<string> = ref('');
const apps: Ref<ConnectApp[]> = ref([]);
const running: Ref<ConnectRunningApp[]> = ref([]);
const filter = ref('');
const showSystem = ref(false);
const loading = ref(false);
const errorMessage = ref('');
const leaving = ref(false);

const { phoneIcon, appIcon, refreshIcon } = useIcons({
	phoneIcon: 'smartphone',
	appIcon: 'application-x-executable',
	refreshIcon: 'view-refresh',
});

const device = computed(() => devices.value.find((d) => d.serial === selected.value));

/**
 * System apps are hidden by default. A phone reports around 130 applications
 * and a third of them are things like "Bluetooth settings" — mixed into the
 * same list they turn a menu into a search problem.
 */
const visibleApps = computed(() => {
	const query = filter.value.trim().toLowerCase();
	return apps.value
		.filter((app) => showSystem.value || !app.system)
		.filter((app) => !query || app.label.toLowerCase().includes(query))
		.sort((a, b) => a.label.localeCompare(b.label));
});

const isRunning = (pkg: string) =>
	running.value.some((r) => r.serial === selected.value && r.package === pkg);

const loadApps = async (refresh = false) => {
	if (!selected.value) return;
	loading.value = true;
	errorMessage.value = '';
	try {
		apps.value = await listConnectApps(selected.value, refresh);
	} catch (error) {
		// The most common reason by far is a phone that has not accepted the
		// debugging prompt, and the service says so in words worth showing.
		errorMessage.value = String(error);
		apps.value = [];
	} finally {
		loading.value = false;
	}
};

const loadDevices = async () => {
	devices.value = await listConnectDevices();
	if (!devices.value.some((d) => d.serial === selected.value)) {
		selected.value = devices.value[0]?.serial ?? '';
		apps.value = [];
		if (selected.value) await loadApps();
	}
};

const open = async (app: ConnectApp) => {
	try {
		await launchConnectApp(selected.value, app.package);
		running.value = await listConnectRunning();
	} catch (error) {
		logError('No se pudo abrir la aplicación del teléfono:', error);
		errorMessage.value = String(error);
	}
};

const close = async (app: ConnectApp) => {
	await stopConnectApp(selected.value, app.package);
	running.value = await listConnectRunning();
};

const closeAfterAnimation = () => {
	if (leaving.value) return;
	leaving.value = true;
	setTimeout(() => {
		menuWindow.hide().catch(() => {
			/* already gone */
		});
	}, 200);
};

const onKeydown = (event: KeyboardEvent) => {
	if (event.key === 'Escape') closeAfterAnimation();
};

const unlisteners: UnlistenFn[] = [];

onMounted(async () => {
	await loadDevices();
	running.value = await listConnectRunning();

	// The daemon knows the instant udev does; polling here would be both slower
	// and a permanent cost in the process that is always running.
	for (const [event, handler] of [
		['connect-device-added', loadDevices],
		['connect-device-changed', loadDevices],
		['connect-device-removed', loadDevices],
		[
			'connect-app-closed',
			async () => {
				running.value = await listConnectRunning();
			},
		],
	] as const) {
		unlisteners.push(await listen(event, handler));
	}

	document.addEventListener('keydown', onKeydown);
	menuWindow
		.onFocusChanged(({ payload: focused }) => {
			if (focused) {
				leaving.value = false;
				return;
			}
			closeAfterAnimation();
		})
		.then((fn) => unlisteners.push(fn));
});

onBeforeUnmount(() => {
	document.removeEventListener('keydown', onKeydown);
	for (const un of unlisteners) un();
});
</script>

<template>
  <Transition appear enter-active-class="enter-active">
    <div
      :class="[
        'flex h-screen flex-col gap-3 rounded-corner border border-ui-border bg-ui-bg/80 p-4',
        { 'leave-active': leaving },
      ]"
    >
      <header class="flex items-center gap-3">
        <img :src="phoneIcon" alt="" class="h-8 w-8" />
        <div class="min-w-0 flex-1">
          <select
            v-if="devices.length > 1"
            v-model="selected"
            @change="loadApps()"
            class="w-full rounded-corner bg-ui-surface px-2 py-1 text-sm text-tx-main"
          >
            <option v-for="d in devices" :key="d.serial" :value="d.serial">{{ d.model }}</option>
          </select>
          <p v-else class="truncate font-semibold text-tx-main">
            {{ device?.model || t('views.connect.noDevice') }}
          </p>
          <p v-if="device" class="text-xs text-tx-muted">
            {{ device.transport === 'usb' ? 'USB' : device.address }}
          </p>
        </div>
        <button
          v-if="device?.state === 'ready'"
          type="button"
          :title="t('views.connect.refresh')"
          @click="loadApps(true)"
          class="rounded-corner p-2 hover:bg-primary"
        >
          <img :src="refreshIcon" alt="" class="h-5 w-5" />
        </button>
      </header>

      <!-- A phone that has not been authorised is the single most common
           first-run state, so it gets an explanation rather than an empty list. -->
      <div
        v-if="device && device.state === 'unauthorized'"
        class="rounded-corner bg-status-warning/10 p-4 text-sm text-status-warning"
      >
        {{ t('views.connect.unauthorized') }}
      </div>

      <div v-else-if="!device" class="flex flex-1 items-center justify-center px-6 text-center">
        <p class="text-tx-main/60">{{ t('views.connect.plugIn') }}</p>
      </div>

      <template v-else>
        <input
          v-model="filter"
          type="search"
          :placeholder="t('views.connect.search')"
          class="w-full rounded-corner bg-ui-surface px-3 py-2 text-sm text-tx-main placeholder:text-tx-muted focus:outline-none focus:ring-2 focus:ring-primary"
        />

        <div v-if="loading" class="flex flex-1 items-center justify-center">
          <p class="text-tx-muted">{{ t('views.connect.loading') }}</p>
        </div>

        <div v-else-if="errorMessage" class="rounded-corner bg-status-error/10 p-4 text-sm text-status-error">
          {{ errorMessage }}
        </div>

        <ul v-else class="flex-1 space-y-1 overflow-y-auto">
          <li v-for="app in visibleApps" :key="app.package">
            <div class="group flex items-center gap-3 rounded-corner p-2 hover:bg-primary/20">
              <button type="button" class="flex min-w-0 flex-1 items-center gap-3 text-left" @click="open(app)">
                <img :src="app.icon || appIcon" alt="" class="h-8 w-8 shrink-0" />
                <span class="truncate text-tx-main">{{ app.label }}</span>
              </button>
              <button
                v-if="isRunning(app.package)"
                type="button"
                :title="t('views.connect.close')"
                @click="close(app)"
                class="shrink-0 rounded-corner px-2 py-1 text-xs text-primary hover:bg-primary hover:text-white"
              >
                {{ t('views.connect.close') }}
              </button>
            </div>
          </li>
          <li v-if="visibleApps.length === 0" class="py-8 text-center text-tx-muted">
            {{ t('views.connect.noApps') }}
          </li>
        </ul>

        <label class="flex items-center gap-2 text-xs text-tx-muted">
          <input v-model="showSystem" type="checkbox" class="accent-primary" />
          {{ t('views.connect.showSystem') }}
        </label>
      </template>
    </div>
  </Transition>
</template>
