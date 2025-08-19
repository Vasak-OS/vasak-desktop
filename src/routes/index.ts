import { createWebHashHistory, createRouter } from "vue-router";

import ControlCenterView from "@/views/ControlCenterView.vue";
import DesktopView from "../views/DesktopView.vue";
import MenuView from "../views/MenuView.vue";
import PanelView from "../views/PanelView.vue";

const routes = [
  { path: "/desktop", component: DesktopView },
  { path: "/panel", component: PanelView },
  { path: "/menu", component: MenuView },
  { path: "/control_center", component: ControlCenterView },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
