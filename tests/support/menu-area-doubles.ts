/**
 * El doble de lo que importa `MenuArea.vue` desde `@/…`: la tarjeta de cada
 * aplicación, una caja con el nombre. Lo que se mira es la lista, no la tarjeta.
 */
import { defineComponent, h } from 'vue';

export default defineComponent({
	props: { app: { type: Object, required: true } },
	setup: (props) => () => h('div', { class: 'app-card' }, (props.app as { name: string }).name),
});
