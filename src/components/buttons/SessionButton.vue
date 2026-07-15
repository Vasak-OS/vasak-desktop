<script lang="ts" setup>
defineProps({
	title: String,
	img: String,
});
</script>

<template>
  <a :title="title" href="#" class="theme-transition">
    <img :src="img" :alt="title" />
  </a>
</template>
