import path from "path";
import { createWebHashHistory, createRouter } from "vue-router";

const routes = [
  { path: "/desktop", component: () => import("@/views/DesktopView.vue") },
  { path: "/panel", component: () => import("@/views/PanelView.vue") },
  { path: "/menu", component: () => import("@/views/MenuView.vue") },
  {
    path: "/control_center",
    component: () => import("@/views/ControlCenterView.vue"),
  },
  {
    path: "/applets",
    children: [
      {
        path: "bluetooth",
        component: () => import("@/views/applets/BluetoothAppletView.vue"),
      },
    ],
  },
  {
    path: "/apps",
    children: [
      {
        path: "terminal",
        component: () => import("@/views/apps/TerminalView.vue"),
      },
    ],
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
