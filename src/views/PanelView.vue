<script setup lang="ts">
import { onMounted, onUnmounted, ref, type Ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import WindowsArea from '@/components/areas/panel/WindowsArea.vue';
import TrayBarArea from '@/components/areas/panel/TrayBarArea.vue';
import PanelClockwidget from '@/components/widgets/PanelClockwidget.vue';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import menuIcon from '@/assets/img/icon.png';
import { logError } from '@/utils/logger';

const notifyIcon: Ref<string> = ref('');
const configIcon: Ref<string> = ref('');
const fileManagerIcon: Ref<string> = ref('');

const notifications = ref<Notification[]>([]);
const hasNewNotifications = ref(false);
let unlistenNotifications: Ref<(() => void) | null> = ref(null);

const setIcons = async () => {
	try {
		notifyIcon.value = await getIconSource('preferences-desktop-notification');
		configIcon.value = await getIconSource('preferences-system');
		fileManagerIcon.value = await getIconSource('system-file-manager');
	} catch (err) {
		logError('Error finding icons:', err);
	}
};

const openMenu = async () => {
	try {
		await invoke('toggle_menu');
	} catch (error) {
		logError('Error al abrir el menu:', error);
	}
};

const openConfig = async () => {
	try {
		await invoke('toggle_config_app');
	} catch (error) {
		logError('Error al abrir config:', error);
	}
};

const openFileManager = async () => {
	try {
		await invoke('open_file_manager_window');
	} catch (error) {
		logError('Error al abrir file manager:', error);
	}
};

const openNotificationCenter = async () => {
	try {
		await invoke('toggle_control_center');
	} catch (error) {
		logError('Error al abrir el centro de control:', error);
	}
};

async function loadNotifications() {
	try {
		notifications.value = await invoke('get_all_notifications');
	} catch (error) {
		logError('Error loading notifications:', error);
	}
}

onMounted(async () => {
	setIcons();
	await loadNotifications();

	unlistenNotifications.value = await listen(
		'notifications-updated',
		(event) => {
			const newNotifications = event.payload as Notification[];
			hasNewNotifications.value =
        newNotifications.length > notifications.value.length;
			notifications.value = newNotifications;

			// Reset animation after a short delay
			if (hasNewNotifications.value) {
				setTimeout(() => {
					hasNewNotifications.value = false;
				}, 1000);
			}
		}
	);
});

onUnmounted(() => {
	if (unlistenNotifications.value) {
		unlistenNotifications.value();
	}
});
</script>

<template>
  <nav class="vpanel background">
    <div class="flex items-center gap-1">
      <img :src="menuIcon" alt="Menu" @click="openMenu" class="app-icon" />
      <img
        :src="configIcon"
        alt="Config"
        @click="openConfig"
        class="app-icon"
      />
      <img
        :src="fileManagerIcon"
        alt="Files"
        @click="openFileManager"
        class="app-icon"
      />
    </div>
    <WindowsArea />
    <div class="flex content-center items-center">
      <TrayBarArea />
      <PanelClockwidget />
      <div class="notification-icon-wrapper" @click="openNotificationCenter">
        <img
          :src="notifyIcon"
          alt="Notifications"
          class="app-icon"
          :class="{ 'bell-shake': hasNewNotifications }"
        />
        <div v-if="notifications.length > 0" class="notification-badge">
          {{ notifications.length > 99 ? "99+" : notifications.length }}
        </div>
      </div>
    </div>
  </nav>
</template>

<style>
@reference "../style.css";

.vpanel {
  @apply flex justify-between items-center mx-1;
  height: 34px;
  padding: 4px;
  border-radius: calc(var(--border-radius) + 2px);
  margin-top: 4px;
}

.vpanel .app-icon {
  @apply h-6 w-6 cursor-pointer p-0.5 rounded-vsk hover:bg-vsk-primary/30 transform hover:scale-110 active:scale-95 ease-in-out;
}

.notification-icon-wrapper {
  @apply relative cursor-pointer;
}

.notification-badge {
  @apply absolute -top-0.5 -right-0.5 bg-vsk-primary text-white rounded-full min-w-3 h-3 flex items-center justify-center;
  font-size: 8px;
  font-weight: 600;
  line-height: 1;
  padding: 0 2px;
  box-shadow: 0 0 0 1px white, 0 1px 2px rgba(0, 0, 0, 0.2);
  border: 1px solid rgba(255, 255, 255, 0.3);
}

@keyframes bell-shake {
  0%,
  100% {
    transform: rotate(0deg);
  }
  10%,
  30%,
  50%,
  70%,
  90% {
    transform: rotate(-10deg);
  }
  20%,
  40%,
  60%,
  80% {
    transform: rotate(10deg);
  }
}

.bell-shake {
  animation: bell-shake 0.8s ease-in-out;
}
</style>
