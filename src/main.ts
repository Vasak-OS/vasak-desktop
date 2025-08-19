import { createApp } from "vue";
import { router } from "./routes/index";
import { createPinia } from "pinia";

import App from "./App.vue";
import "./style.css";

const pinia = createPinia();
const app = createApp(App);

app.use(router);
app.use(pinia);

app.mount("#app");
