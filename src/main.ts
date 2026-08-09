import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import { router } from '@/routes/index';
import '@/assets/main.css';

const pinia = createPinia();
const i18n = I18n.getInstance();
const app = createApp(App);

i18n.load();
app.use(pinia);
app.use(router);

app.mount('#app');
