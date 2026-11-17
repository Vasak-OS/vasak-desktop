/**
 * Lo que importa `NotificationCard.vue` desde `@/…`.
 *
 * El servicio es el de verdad: llama `invoke`, y el `invoke` de mentira de
 * `mount-sfc.ts` anota el comando y los argumentos que llegan al borde de Tauri.
 */
export { invokeNotificationAction } from '../../src/services/notification.service';

export function logError(): void {}
