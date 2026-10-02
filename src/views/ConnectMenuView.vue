<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	EmptyState,
	LoadingState,
	SearchField,
	SegmentedControl,
	SettingRow,
	SwitchToggle,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onBeforeUnmount, onMounted, type Ref, ref } from 'vue';
import ConnectAppButton from '@/components/buttons/ConnectAppButton.vue';
import type { ConnectApp, ConnectDevice, ConnectRunningApp } from '@/interfaces/connect';
import {
	launchConnectApp,
	listConnectApps,
	listConnectDevices,
	listConnectRunning,
	stopConnectApp,
} from '@/services/connect.service';
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

const device = computed(() => devices.value.find((d) => d.serial === selected.value));

/** Los teléfonos enchufados, como opciones del selector cuando hay más de uno. */
const deviceOptions = computed(() =>
	devices.value.map((d) => ({ value: d.serial, label: d.model }))
);

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
	const previous = device.value?.state;
	devices.value = await listConnectDevices();

	if (!devices.value.some((d) => d.serial === selected.value)) {
		selected.value = devices.value[0]?.serial ?? '';
		apps.value = [];
		if (selected.value) await loadApps();
		return;
	}

	// The phone was waiting for the debugging prompt and has just been allowed.
	// Without this the list stays empty behind a notice that is no longer true:
	// nothing else would ask again, because the window is already open.
	if (previous !== 'ready' && device.value?.state === 'ready' && apps.value.length === 0) {
		await loadApps();
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
				// Reopened after being hidden: the phone may have been unplugged
				// or authorised while nobody was looking at this window.
				void loadDevices();
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
        'flex h-screen min-w-0 flex-col gap-3 rounded-corner-m border border-ui-line bg-ui-shell p-4 text-tx-main',
        { 'leave-active': leaving },
      ]"
    >
      <header class="flex min-w-0 items-center gap-3">
        <ThemeIcon name="smartphone" :size="32" />
        <div class="min-w-0 flex-1">
          <!-- A native <select> is drawn by GTK, not by the stylesheet, so its
               popup ignores the session's colours entirely. With one phone —
               the normal case — there is nothing to choose anyway. -->
          <SegmentedControl
            v-if="devices.length > 1"
            :model-value="selected"
            :options="deviceOptions"
            :label="t('views.connect.chooseDevice')"
            variant="chips"
            @change="(serial) => { selected = serial; loadApps(); }"
          />
          <p v-else class="truncate font-semibold">
            {{ device?.model || t('views.connect.noDevice') }}
          </p>
          <p v-if="device" class="truncate text-label-xs text-tx-muted">
            {{ device.transport === 'usb' ? 'USB' : device.address }}
          </p>
        </div>
        <ActionButton
          v-if="device?.state === 'ready'"
          label=""
          icon="view-refresh"
          :icon-alt="t('views.connect.refresh')"
          :title="t('views.connect.refresh')"
          variant="ghost"
          @click="loadApps(true)"
        />
      </header>

      <!-- A phone that has not been authorised is the single most common
           first-run state, so it gets an explanation rather than an empty list. -->
      <AlertMessage v-if="device && device.state === 'unauthorized'" tone="warning">
        {{ t('views.connect.unauthorized') }}
      </AlertMessage>

      <EmptyState
        v-else-if="!device"
        class="flex-1"
        :title="t('views.connect.plugIn')"
        icon="smartphone"
      />

      <template v-else>
        <SearchField v-model="filter" :label="t('views.connect.search')" class="w-full" />

        <LoadingState v-if="loading" class="flex-1" :label="t('views.connect.loading')" />

        <AlertMessage v-else-if="errorMessage" tone="error">
          {{ errorMessage }}
        </AlertMessage>

        <ul v-else class="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
          <li v-for="app in visibleApps" :key="app.package">
            <ConnectAppButton :app="app" :running="isRunning(app.package)" @open="open(app)">
              <template v-if="isRunning(app.package)" #actions>
                <ActionButton
                  :label="t('views.connect.close')"
                  variant="secondary"
                  size="sm"
                  stop-propagation
                  @click="close(app)"
                />
              </template>
            </ConnectAppButton>
          </li>
          <li v-if="visibleApps.length === 0">
            <EmptyState :title="t('views.connect.noApps')" icon="" size="sm" />
          </li>
        </ul>

        <SettingRow :label="t('views.connect.showSystem')">
          <SwitchToggle :label="t('views.connect.showSystem')" :model-value="showSystem" @update:model-value="showSystem = $event" />
        </SettingRow>
      </template>
    </div>
  </Transition>
</template>

<style scoped>
/* The same open and close animation as the application menu. Defined here
   because MenuView's copy is scoped to that component, so referencing its class
   names from another view silently produced no animation at all. */
@keyframes scale-in {
  from {
    transform: scale(0.95);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

@keyframes scale-out {
  from {
    transform: scale(1);
    opacity: 1;
  }
  to {
    transform: scale(0.95);
    opacity: 0;
  }
}

.enter-active {
  animation: scale-in 200ms ease-out;
}

.leave-active {
  animation: scale-out 200ms ease-in;
}
</style>
