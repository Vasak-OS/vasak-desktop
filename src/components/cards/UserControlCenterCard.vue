<template>
  <div
    class="bg-ui-bg/80 rounded-corner border border-ui-border p-4 flex items-center gap-4 w-full transition-all duration-300 hover:bg-secondary hover:scale-[1.02] group"
    :class="{
      'opacity-0 translate-y-4': !isLoaded,
      'opacity-100 translate-y-0': isLoaded,
    }"
  >
    <div
      class="relative w-16 h-16 rounded-full transition-all duration-300 group-hover:scale-110 "
    >
      <img
        :src="userInfo.avatar_data"
        :alt="userInfo.full_name"
        class="h-full w-full aspect-square object-cover transition-all duration-300 "
        :class="{
          'opacity-0 scale-90': !isLoaded,
          'opacity-100 scale-100': isLoaded,
        }"
      />

    </div>
    <div class="flex flex-col flex-1 space-y-1">
      <h2
        class="text-lg font-semibold transition-all duration-300 group-hover:text-primary"
      >
        {{ userInfo.full_name }}
      </h2>
      <p
        class="text-sm text-tx-muted transition-all duration-500 group-hover:opacity-90"
      >
        {{ userInfo.username }}
      </p>
    </div>
    <div
      class="text-right space-y-1 transition-all duration-700"
    >
      <div
        class="text-2xl font-medium transition-all duration-300 tabular-nums text-primary"
        :class="{ 'animate-pulse': isTimeUpdating }"
      >
        {{ currentTime }}
      </div>
      <div
        class="text-sm text-tx-muted transition-all duration-300 capitalize"
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

