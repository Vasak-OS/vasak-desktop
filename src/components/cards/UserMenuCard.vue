<script setup lang="ts">
import { ref, onMounted, computed, Ref } from "vue";
import { getUserData, type UserInfo } from "@vasakgroup/plugin-user-data";

const userInfo: Ref<UserInfo | null> = ref<UserInfo | null>(null);

const avatarSrc = computed(() => {
  return userInfo.value?.avatar_data;
});

const loadUserInfo = async () => {
  try {
    userInfo.value = await getUserData();
  } catch (error) {
    console.error("Error al cargar información del usuario:", error);
  }
};

onMounted(loadUserInfo);
</script>

<template>
  <div v-if="userInfo" class="flex items-center space-x-3 backdrop-blur-sm">
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
