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
  <!-- El avatar va en `rounded-corner-full`, que sale del radio que eligió la
       persona (con radio 0 es cuadrado, igual que el resto), y con el canto
       fino del esquema: el acento queda para lo que actúa. El nombre no se
       corta: se parte. -->
  <div v-if="userInfo" class="flex min-w-0 items-center gap-3">
    <img
      :src="avatarSrc"
      :alt="userInfo.username"
      class="size-10 shrink-0 rounded-corner-full border border-ui-line object-cover"
    />
    <div class="flex min-w-0 flex-col">
      <span class="break-words font-semibold text-label-m text-tx-main">{{ userInfo.full_name }}</span>
      <span class="break-words text-label-xs text-tx-muted">@{{ userInfo.username }}</span>
    </div>
  </div>
</template>
