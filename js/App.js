/* eslint-disable no-undef */
import { createApp } from 'vue';
import ClockWidget from './components/ClockWidget.js';

/* Init APP VueJS */
const app = createApp({
	data() {
		return {
			background: `file://${this.$homePath}.background.video`,
		};
	},
	async beforeMount() {
		// Set windows Properties
		this.$win.setShowInTaskbar(false);
		this.$win.resizeTo(
			Math.round(this.$screen.width),
			Math.round(this.$screen.height)
		);
		this.$win.setResizable(false);
		this.$win.y = 0;
		this.$win.x = 0;
		this.$win.setVisibleOnAllWorkspaces(true);
		/*this.$exec('python', [
			'/usr/share/vasak-desktop-service/setters/set_desktop.py',
			this.$pid.toString(),
		]);*/ //deprecated
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
app.config.globalProperties.$execSynx = require('child_process').spawnSync;
app.config.globalProperties.$exec = require('child_process').spawn;
app.config.globalProperties.$win = nw.Window.get();
app.config.globalProperties.$process = process;
app.config.globalProperties.$pid = process.pid;
app.config.globalProperties.$homePath = nw.App.dataPath.concat('/../../../');
app.config.globalProperties.$screen = window.screen;

app.mount('#app');
