/* eslint-disable no-undef */
import { createApp } from 'vue';

/* Init APP VueJS */
const app = createApp({
	data() {
		return {
			background: `file://${homePath}.background.video`
		};
	},
	async beforeMount() {
		// Set windows Properties
		this.$win.setShowInTaskbar(false);
		this.$win.resizeTo(Math.round(this.$screen.width), Math.round(this.$screen.height));
		this.$win.setResizable(false);
		this.$win.y = 0;
		this.$win.x = 0;
		this.$win.setVisibleOnAllWorkspaces(true);
		this.$exec('python', [
			'/usr/share/Lynx/lynx-desktop-service/Setters/setDesktop.py',
			`${this.$pid.toString()}`
		]);
	},
	template: `
    <video
        style="border-radius:0px;"
        id="video-background"
        type="video/mp4"
        :src="background"
        loop
        autoplay
        muted>
	</video>`,
	components: {
		MenuButtom,
		NotificationArea,
		WindowsSections,
	},
});

/* Add Services aditional to VueJS in Global Properties */
app.config.globalProperties.$execSynx = require('child_process').spawnSync;
app.config.globalProperties.$exec = require('child_process').spawn;
app.config.globalProperties.$systeminformation = require('systeminformation');
app.config.globalProperties.$win = nw.Window.get();
app.config.globalProperties.$process = process;
app.config.globalProperties.$pid = process.pid;
app.config.globalProperties.$homePath = nw.App.dataPath.concat('/../../../');

app.mount('#app');