import { getIconSource } from '@vasakgroup/plugin-vicons';
import { setupContextMenu } from '@vasakgroup/plugin-vsk-contextual-menu';
import I18n from '@vasakgroup/tauri-plugin-i18n';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from '@/App.vue';
import { router } from '@/routes/index';
import { sanearUrl } from '@/tools/csp';
import '@/assets/main.css';
import { captureFailures } from '@vasakgroup/plugin-vsk-journal';

// Una violación de CSP no se ve: el recurso no carga y la interfaz queda a
// medias sin decir nada. Se sanean **las dos** URLs, porque `sourceFile` también
// puede llevar query con datos sensibles.
document.addEventListener('securitypolicyviolation', (evento) => {
	// El respaldo va **después** de sanear, no antes.
	//
	// Mirando el valor crudo, una entrada como `?token=X` es verdadera y
	// pasa el respaldo de largo — pero lo que queda de ella al sanearla es
	// nada, así que el registro salía con el campo en blanco. Sanear
	// primero y decidir después es lo que hace que un aviso incompleto no
	// exista.
	const recurso = sanearUrl(evento.blockedURI) || '(en línea)';
	const origen = sanearUrl(evento.sourceFile) || 'documento';
	console.error(
		`[CSP] bloqueado ${recurso} por la directiva ` +
			`«${evento.violatedDirective}» en ${origen}:${evento.lineNumber}`
	);
});

// El menú del clic derecho de todo el escritorio. Enseñarle a resolver nombres
// de iconos del sistema es una línea, y a partir de ahí los ítems pueden nombrar
// iconos como `preferences-system` en vez de pasar una imagen ya armada.
setupContextMenu({ iconResolver: getIconSource });

const pinia = createPinia();
const i18n = I18n.getInstance();
// Lo que rompe la interfaz va al diario del sistema, con el nombre de esta
// aplicación. Antes no iba a ninguna parte: un error de JavaScript deja la
// pantalla a medias y la consola del WebView no la ve nadie en una máquina
// instalada.
captureFailures();

const app = createApp(App);

i18n.load();
app.use(pinia);
app.use(router);

app.mount('#app');
