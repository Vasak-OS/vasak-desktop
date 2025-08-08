import { createWebHashHistory, createRouter } from "vue-router";

import DesktopView from "../views/DesktopView.vue";
import MenuView from "../views/MenuView.vue";
import PanelView from "../views/PanelView.vue";

const routes = [
  { path: "/desktop", component: DesktopView },
  { path: "/panel", component: PanelView },
  { path: "/menu", component: MenuView },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
