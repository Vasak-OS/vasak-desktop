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

/**
 * Cuánto se espera a las traducciones antes de montar.
 *
 * Se espera para que la primera pantalla no muestre las claves crudas, pero con
 * un plazo: si el backend no contesta, es mejor una interfaz con las claves a la
 * vista que una ventana en blanco para siempre.
 */
const PLAZO_TRADUCCIONES_MS = 3000;

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

app.use(pinia);
app.use(router);

// Se esperan las traducciones antes de montar: montando primero, la ventana
// enseña las claves crudas —«views.home.title» donde va el texto— hasta que el
// catálogo termina de cargar.
//
// Antes esto era un `i18n.load()` suelto, sin esperar. Y hasta la 2.3.0 del
// plugin esperarlo tampoco habría servido: la clase y el composable guardaban el
// catálogo por separado, y el `t()` de los componentes lee el del composable.
//
// Un intento que falla se reintenta —el backend puede tardar o no escuchar a la
// primera—, pero la espera total sigue acotada por `PLAZO_TRADUCCIONES_MS`: un
// backend colgado tiene que dar una ventana con las claves a la vista, no una
// ventana en blanco para siempre. Ese tope era la garantía del `Promise.race`
// anterior, y el reintento no la toca.
async function cargarTraducciones(): Promise<void> {
	const MAX_INTENTOS = 3;
	const ESPERA_BASE_MS = 500;
	const ESPERA_MAX_MS = 2000;

	const intentar = async () => {
		for (let intento = 0; intento < MAX_INTENTOS; intento++) {
			try {
				await i18n.load();
				return;
			} catch (error) {
				console.error(
					`No se pudieron cargar las traducciones (intento ${intento + 1}/${MAX_INTENTOS}):`,
					error
				);
				if (intento === MAX_INTENTOS - 1) return;
				const espera = Math.min(ESPERA_BASE_MS * 2 ** intento, ESPERA_MAX_MS);
				await new Promise((resolve) => setTimeout(resolve, espera));
			}
		}
	};

	await Promise.race([
		intentar(),
		new Promise((resolve) => setTimeout(resolve, PLAZO_TRADUCCIONES_MS)),
	]);
}

await cargarTraducciones();

app.mount('#app');
