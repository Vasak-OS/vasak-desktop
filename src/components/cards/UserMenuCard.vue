<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { getUserData, type UserInfo } from '@vasakgroup/plugin-user-data';
import { computed, onMounted, type Ref, ref } from 'vue';
import { logError } from '@/utils/logger';

const userInfo: Ref<UserInfo | null> = ref<UserInfo | null>(null);

const avatarSrc = computed(() => {
	return userInfo.value?.avatar_data;
});

const loadUserInfo = async () => {
	try {
		userInfo.value = await getUserData();
	} catch (error) {
		logError('Error al cargar información del usuario:', error);
	}
};

onMounted(loadUserInfo);
</script>

<template>
  <div v-if="userInfo" class="flex items-center space-x-3">
    <img
      :src="avatarSrc"
      :alt="userInfo.username"
      class="w-10 h-10 rounded-full object-cover"
    />
    <div class="flex flex-col">
      <span class="text-sm font-bold">{{ userInfo.full_name }}</span>
      <span class="text-xs text-white/70">@{{ userInfo.username }}</span>
    </div>
  </div>
</template>
