/* eslint-disable no-undef */

/* Init APP VueJS */
const app = Vue.createApp({
	data() {
		return {
			background: `file://${this.$homePath}.background.video`,
		};
	},
	template: `
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
	components: {
		ClockWidget,
	},
});

/* Add Services aditional to VueJS in Global Properties */
app.config.globalProperties.$homePath = '/home/pato/';

app.mount('#app');
