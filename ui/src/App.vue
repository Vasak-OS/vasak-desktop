<script lang="ts">
import ClockWidget from "./components/widgets/ClockWidgert.vue";
//import background from "./assets/background.mp4";

const background = '/home/pato/.backgroud.mp4';
export default {
	name: "App",
	components: {
		ClockWidget,
	},
	data() {
		return {
			background: '',
		};
	},
	methods: {
		async setBackground() : Promise<void>  {
			this.background = await (this as any).$vsk.getHome() + '/.config/vsk/background.mp4';
		},
	},
	mounted() {
		this.setBackground();	
	},
};
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
	</div>
</template>

<style>
body,
#app,
.global {
	overflow: auto;
	width: 100vw;
	height: 100vh;
}
</style>