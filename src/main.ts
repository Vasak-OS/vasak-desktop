import { getIconSource } from '@vasakgroup/plugin-vicons';
import { setupContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import { router } from '@/routes/index';
import '@/assets/main.css';

// Una violación de CSP no se ve: el recurso simplemente no carga y la interfaz
// queda a medias sin decir nada. Esto la manda a la consola, que es donde se
// puede encontrar al ajustar la política.
document.addEventListener('securitypolicyviolation', (evento) => {
	// Sin la query ni el fragmento: `blockedURI` puede llevar tokens o
	// identificadores, y en esta aplicación el `console.error` va al registro en
	// disco. Para saber qué directiva falló alcanza el origen y la ruta.
	let recurso = evento.blockedURI || '(en línea)';
	try {
		const url = new URL(recurso);
		recurso = url.protocol === 'data:' ? 'data:(recortado)' : `${url.origin}${url.pathname}`;
	} catch {
		// No era una URL absoluta —'inline', 'eval', una ruta relativa—: va tal cual.
	}
	console.error(
		`[CSP] bloqueado ${recurso} por la directiva ` +
			`«${evento.violatedDirective}» en ${evento.sourceFile ?? 'documento'}:${evento.lineNumber}`
	);
});

// El menú del clic derecho de todo el escritorio. Enseñarle a resolver nombres
// de iconos del sistema es una línea, y a partir de ahí los ítems pueden nombrar
// iconos como `preferences-system` en vez de pasar una imagen ya armada.
setupContextMenu({ iconResolver: getIconSource });

const pinia = createPinia();
const i18n = I18n.getInstance();
const app = createApp(App);

i18n.load();
app.use(pinia);
app.use(router);

app.mount('#app');
