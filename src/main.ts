import { getIconSource } from '@vasakgroup/plugin-vicons';
import { setupContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import { router } from '@/routes/index';
import '@/assets/main.css';

// El menú del clic derecho de todo el escritorio. Enseñarle a resolver nombres
// de iconos del sistema es una línea, y a partir de ahí los ítems pueden nombrar
// iconos como `preferences-system` en vez de pasar una imagen ya armada.
setupContextMenu({ iconResolver: getIconSource });

const pinia = createPinia();
const i18n = I18n.getInstance();
const app = createApp(App);

i18n.load();
app.use(pinia);
app.use(router);

app.mount('#app');
