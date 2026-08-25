import { getIconSource } from '@vasakgroup/plugin-vicons';
import { setupContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import { router } from '@/routes/index';
import '@/assets/main.css';

/**
 * Saca de una URL lo que no debería quedar en un registro.
 *
 * Se conserva el esquema y la autoridad completos usando `href`, y no
 * `origin + pathname`: para esquemas propios como `asset:` o `ipc:` el `origin`
 * es la cadena «null», así que esa forma escribía `null/ruta` y perdía
 * justamente lo que permite entender qué se bloqueó.
 */
const sanearUrl = (valor: string | null | undefined): string => {
	if (!valor) {
		return '(en línea)';
	}
	try {
		const url = new URL(valor);
		if (url.protocol === 'data:') {
			return 'data:(recortado)';
		}
		// Credenciales, query y fragmento: ahí es donde viajan los tokens.
		url.username = '';
		url.password = '';
		url.search = '';
		url.hash = '';
		return url.href;
	} catch {
		// No era una URL absoluta —'inline', 'eval', una ruta relativa—: tal cual.
		return valor;
	}
};

document.addEventListener('securitypolicyviolation', (evento) => {
	// Se sanean **las dos** URLs. `sourceFile` también puede llevar query con
	// datos sensibles, y antes se escribía sin tocar.
	console.error(
		`[CSP] bloqueado ${sanearUrl(evento.blockedURI)} por la directiva ` +
			`«${evento.violatedDirective}» en ${sanearUrl(evento.sourceFile) || 'documento'}:${evento.lineNumber}`
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
