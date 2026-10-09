/**
 * `MarqueeText.vue` sólo importa el ayudante puro del desplazamiento desde
 * `@/…`: acá se reexporta el de verdad, que es lo que se quiere probar.
 */
export { marqueeShift } from '../../src/tools/marquee';
