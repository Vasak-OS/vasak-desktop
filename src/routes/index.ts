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
      {
        path: "network",
        component: () => import("@/views/applets/NetworkAppletView.vue"),
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
      {
        path: "configuration",
        redirect: "/apps/configuration/info",
        children: [
          {
            path: "info",
            component: () =>
              import("@/views/apps/configuration/ConfigInfoView.vue"),
          },
          {
            path: "network",
            component: () =>
              import("@/views/apps/configuration/ConfigNetworkView.vue"),
          },
          {
            path: "bluetooth",
            component: () =>
              import("@/views/apps/configuration/ConfigBluetoothView.vue"),
          },
          {
            path: "style",
            component: () =>
              import("@/views/apps/configuration/ConfigStyleView.vue"),
          },
        ],
      },
    ],
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
