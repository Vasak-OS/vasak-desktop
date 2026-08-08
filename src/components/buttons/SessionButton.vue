<script lang="ts" setup>
// A plain <button>, not an <a href="#">.
//
// The router uses hash history, so clicking an href="#" anchor cleared
// location.hash, navigated to a path with no matching route, and left the menu
// window rendering nothing.
defineProps({
	title: String,
	img: String,
});
</script>

<template>
  <button type="button" :title="title" class="theme-transition">
    <img :src="img" :alt="title" />
  </button>
</template>
