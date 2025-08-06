import { createWebHashHistory, createRouter } from "vue-router";

import DesktopView from "../views/DesktopView.vue";
import PanelView from "../views/PanelView.vue";

const routes = [
  { path: "/desktop", component: DesktopView },
  { path: "/panel", component: PanelView },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
