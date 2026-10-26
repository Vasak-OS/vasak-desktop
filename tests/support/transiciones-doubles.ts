/**
 * Lo que importan `SessionPopupView.vue` y `ConnectMenuView.vue` desde `@/…`.
 *
 * `loadComponent` manda todos los `@/…` a este módulo: el `default` es lo que
 * recibe un `import X from '@/…vue'`, y el resto, los servicios por nombre.
 */
import { defineComponent, h } from 'vue';

export default defineComponent({ name: 'ConnectAppButton', render: () => h('i') });

export const detectDisplayServer = async () => 'wayland';
export const logout = async () => {};
export const reboot = async () => {};
export const shutdown = async () => {};
export const suspend = async () => {};

export const listConnectDevices = async () => [];
export const listConnectApps = async () => [];
export const listConnectRunning = async () => [];
export const launchConnectApp = async () => 0;
export const stopConnectApp = async () => true;

export const logError = () => {};
