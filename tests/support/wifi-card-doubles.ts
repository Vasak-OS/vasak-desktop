/**
 * Lo que importa `NetworkWiFiCard.vue` desde `@/…` (el applet de Wi-Fi).
 *
 * El servicio es **el de verdad**: llama `invoke`, y el `invoke` de mentira de
 * la prueba anota qué comando y qué argumentos llegaron al plugin de red. Así
 * se ve lo que de verdad cruza hacia Rust, no una función doblada.
 */
export { connectToWifi } from '../../src/services/network.service';
