<template>
  <div
    class="background rounded-vsk p-4 flex items-center gap-4 w-full transition-all duration-300 hover:bg-white/60 dark:hover:bg-black/60 hover:shadow-lg hover:scale-[1.02] group"
    :class="{
      'opacity-0 translate-y-4': !isLoaded,
      'opacity-100 translate-y-0': isLoaded,
    }"
  >
    <div
      class="relative w-16 h-16 transition-all duration-300 group-hover:scale-110"
    >
      <img
        :src="userInfo.avatar_data"
        :alt="userInfo.full_name"
        class="w-full h-full rounded-full object-cover transition-all duration-300 group-hover:shadow-xl group-hover:ring-4 group-hover:ring-blue-500/30 avatar-dynamic-ring"
        :class="{
          'opacity-0 scale-90': !isLoaded,
          'opacity-100 scale-100': isLoaded,
        }"
        :style="{
          '--ring-color': getCurrentRingColor(),
        }"
      />
      <div
        class="absolute inset-0 rounded-full bg-lineal-to-tr from-transparent to-white/20 opacity-0 group-hover:opacity-100 transition-opacity duration-300"
      ></div>
    </div>
    <div class="flex flex-col flex-1 space-y-1">
      <h2
        class="text-lg font-semibold transition-all duration-300 group-hover:text-blue-600 dark:group-hover:text-blue-400"
        :class="{
          'opacity-0 translate-x-4': !isLoaded,
          'opacity-100 translate-x-0': isLoaded,
        }"
      >
        {{ userInfo.full_name }}
      </h2>
      <p
        class="text-sm opacity-75 transition-all duration-500 group-hover:opacity-90"
        :class="{
          'opacity-0 translate-x-4': !isLoaded,
          'opacity-75 translate-x-0': isLoaded,
        }"
      >
        {{ userInfo.username }}
      </p>
    </div>
    <div
      class="text-right space-y-1 transition-all duration-700"
      :class="{
        'opacity-0 translate-x-4': !isLoaded,
        'opacity-100 translate-x-0': isLoaded,
      }"
    >
      <div
        class="text-2xl font-medium transition-all duration-300 tabular-nums group-hover:text-green-600 dark:group-hover:text-green-400"
        :class="{ 'animate-pulse': isTimeUpdating }"
      >
        {{ currentTime }}
      </div>
      <div
        class="text-sm opacity-75 transition-all duration-300 group-hover:opacity-90 capitalize"
      >
        {{ currentDate }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { getUserData, type UserInfo } from '@vasakgroup/plugin-user-data';
import { onMounted, onUnmounted, ref } from 'vue';
import { logError } from '@/utils/logger';

const userInfo = ref<UserInfo>({
	username: '',
	full_name: '',
	avatar_data: '',
});

const currentTime = ref('');
const currentDate = ref('');
const isTimeUpdating = ref(false);
const isLoaded = ref(false);

const _getCurrentRingColor = () => {
	const hour = new Date().getHours();
	if (hour >= 6 && hour < 12) return '250, 204, 21';
	if (hour >= 12 && hour < 18) return '251, 146, 60';
	if (hour >= 18 && hour < 22) return '168, 85, 247';
	return '96, 165, 250';
};

const updateDateTime = () => {
	isTimeUpdating.value = true;

	const now = new Date();
	const newTime = now.toLocaleTimeString('es-ES', {
		hour: '2-digit',
		minute: '2-digit',
	});
	const newDate = now.toLocaleDateString('es-ES', {
		weekday: 'long',
		day: 'numeric',
		month: 'long',
	});

	if (currentTime.value !== newTime) {
		currentTime.value = newTime;
	}
	if (currentDate.value !== newDate) {
		currentDate.value = newDate;
	}

	setTimeout(() => {
		isTimeUpdating.value = false;
	}, 200);
};

const getUserInfo = async () => {
	try {
		const info = await getUserData();
		userInfo.value = info as UserInfo;
	} catch (error) {
		logError('Error obteniendo información de usuario:', error);
	}
};

let timeInterval: ReturnType<typeof setInterval>;

onMounted(async () => {
	await getUserInfo();
	updateDateTime();

	setTimeout(() => {
		isLoaded.value = true;
	}, 100);

	timeInterval = globalThis.setInterval(updateDateTime, 1000);
});

onUnmounted(() => {
	if (timeInterval) {
		clearInterval(timeInterval);
	}
});
</script>

<style scoped>
.avatar-dynamic-ring:hover {
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1),
    0 10px 10px -5px rgba(0, 0, 0, 0.04), 0 0 0 4px rgba(var(--ring-color), 0.3);
}

@keyframes gentle-pulse {
  0%,
  100% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.01);
  }
}

.group:hover {
  animation: gentle-pulse 2s ease-in-out infinite;
}

.tabular-nums:hover {
  text-shadow: 0 0 8px currentColor;
}

.group {
  animation: slideInUp 0.6s ease-out;
}

@keyframes slideInUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
