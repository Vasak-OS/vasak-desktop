<script setup lang="ts">
import ClockWidget from "./components/widgets/ClockWidgert.vue";
import background from "./assets/background.mp4";
</script>

<template>
  <div class="global">
		<video
			style="border-radius:0px;"
			id="video-background"
			type="video/mp4"
			:src="background"
			loop
			autoplay
			muted>
		</video>
		<ClockWidget />
	</div>`,
</template>
