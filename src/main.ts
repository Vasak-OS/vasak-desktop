import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from './App.vue';
import { router } from './routes/index';
import './style.css';

// Importar el sistema de logging
import { logInfo } from './utils/logger';

const pinia = createPinia();
const app = createApp(App);

app.use(pinia);
app.use(router);

// Log de inicio
logInfo('Aplicación Vue montada correctamente');

app.mount('#app');
