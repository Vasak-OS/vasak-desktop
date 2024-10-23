<script lang="ts" setup>
import { defineProps } from "vue";

defineProps({
  day: {
    type: Number,
    required: true,
  },
  month: {
    type: Number,
    required: true,
  },
  year: {
    type: Number,
    required: true,
  },
})
</script>

<template>
  <div class="clock-date">
    <span>{{ day }}</span>
    <span>/</span>
    <span>{{ month }}</span>
    <span>/</span>
    <span>{{ year }}</span>
  </div>
</template>
