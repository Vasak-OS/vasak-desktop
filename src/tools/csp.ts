/**
 * Sanear lo que se escribe al registrar una violación de la política de
 * contenido.
 *
 * Una violación de CSP no se ve: el recurso no carga y la interfaz queda a
 * medias sin decir nada. Por eso se registran. Pero lo que se registra es una
 * URL que eligió otro, y las URL llevan credenciales: `//usuario:token@sitio`,
 * `?access_token=…`. Escribirlas al diario las deja ahí para siempre, legibles
 * por cualquiera que lo lea.
 *
 * Este módulo existe aparte de `main.ts` para poder probarlo: importar `main.ts`
 * arranca la aplicación.
 */

/**
 * Lo que la especificación de CSP puede poner en lugar de una URL.
 *
 * No son direcciones, así que no se sanean: se dejan tal cual porque son la
 * información útil del aviso.
 */
export const MARCADORES_CSP = new Set([
	'inline',
	'eval',
	'wasm-eval',
	'data',
	'blob',
	'filesystem',
	'self',
	'unsafe-eval',
	'unsafe-inline',
]);

/**
 * Los esquemas cuyo contenido después de `:` **puede** ser una ruta.
 *
 * «Puede» y no «es»: que el esquema esté en esta lista no alcanza, porque el
 * mismo esquema admite las dos formas. `asset://sitio/x` es jerárquico y
 * `asset:user:token@sitio` es opaco, y el segundo mete el token en lo que la
 * URL llama «ruta», donde limpiar `username` y `password` no hace nada — para
 * un esquema opaco esos campos ni existen. Por eso además se comprueba que la
 * dirección tenga autoridad de verdad.
 *
 * `blob:` está afuera a propósito aunque parezca jerárquico: su contenido es
 * otra URL entera —`blob:https://usuario:token@sitio/x`— y esa no la ve
 * `new URL` como autoridad suya.
 */
const ESQUEMAS_CON_RUTA = new Set([
	'http:',
	'https:',
	'ws:',
	'wss:',
	'ftp:',
	'file:',
	// Los que usa Tauri para servir la aplicación y hablar con el backend.
	'tauri:',
	'asset:',
	'ipc:',
]);

/**
 * Si la dirección tiene autoridad, o sea si su parte después del esquema es
 * una ruta y no una carga útil.
 *
 * `file:` es la excepción: sus direcciones son `file:///ruta`, sin sitio, y no
 * pueden llevar credenciales porque la especificación no se lo permite.
 * Recortarlas perdería la ruta, que es justo lo que sirve para depurar.
 */
function esJerarquica(url: URL): boolean {
	return url.host !== '' || url.protocol === 'file:';
}

/**
 * Si la dirección declara una autoridad, aunque no se haya podido parsear.
 *
 * Se usa para no caer al respaldo con algo que puede llevar credenciales.
 * Mirar sólo si hay un `@` no serviría: una ruta relativa como
 * `/assets/@vite/client.js` tiene uno y es de las más comunes que hay.
 */
const CON_AUTORIDAD = /^(?:[a-z][a-z0-9+.-]*:)?\/\//i;

/** Un esquema inventado para poder parsear una URL sin esquema propio. */
const ESQUEMA_PRESTADO = 'https:';

function sinCredenciales(url: URL): URL {
	url.username = '';
	url.password = '';
	url.search = '';
	url.hash = '';
	return url;
}

/**
 * Saca de una URL lo que no debería quedar en un registro.
 *
 * Devuelve cadena vacía para lo que no tiene nada que registrar —una entrada
 * vacía, o una que era sólo query o fragmento—. Quien la use tiene que aplicar
 * su texto de reserva **después** de llamar acá y no antes: una entrada como
 * `?token=X` no es vacía, pero lo que queda de ella sí.
 */
export function sanearUrl(valor: string | null | undefined): string {
	if (!valor) {
		return '';
	}

	if (MARCADORES_CSP.has(valor)) {
		return valor;
	}

	// Relativas al protocolo: `//usuario:token@sitio/x`. `new URL` sin base las
	// rechaza, y la versión anterior caía a cortar por `?` y `#`, que deja
	// `usuario:token@` intacto. Se parsean con un esquema prestado y se
	// devuelven en la misma forma en que llegaron.
	if (valor.startsWith('//')) {
		try {
			const url = sinCredenciales(new URL(`${ESQUEMA_PRESTADO}${valor}`));
			// `new URL` completa una ruta ausente con «/», y eso cambia la forma
			// de lo que llegó: `//sitio` volvía como `//sitio/`. Se devuelve
			// como vino.
			const sinRuta = url.pathname === '/' && !valor.split(/[?#]/)[0].endsWith('/');
			return sinRuta ? `//${url.host}` : `//${url.host}${url.pathname}`;
		} catch {
			// Ni con esquema prestado. No se cae al respaldo: lo que declara una
			// autoridad puede llevar credenciales, y el respaldo sólo corta la
			// query y el fragmento.
			return '';
		}
	}

	try {
		const url = new URL(valor);
		if (!ESQUEMAS_CON_RUTA.has(url.protocol) || !esJerarquica(url)) {
			return `${url.protocol}(recortado)`;
		}
		return sinCredenciales(url).href;
	} catch {
		// Sin autoridad no puede haber credenciales —requieren una— así que
		// alcanza con quitar la query y el fragmento. Con autoridad no se
		// arriesga: si no se pudo parsear, no se registra.
		return CON_AUTORIDAD.test(valor) ? '' : valor.split(/[?#]/)[0];
	}
}
