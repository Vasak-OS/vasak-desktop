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
 * Los esquemas cuyo contenido después de `:` es una ruta y no una carga útil.
 *
 * Para el resto se registra sólo el esquema. La razón es que `new URL` acepta
 * cualquier cosa con dos puntos como un esquema opaco: `user:token@sitio/x`
 * parsea bien, con `username` **vacío** y el token entero en `href`. O sea que
 * limpiar `username` y `password` no alcanza — para un esquema opaco esos
 * campos ni existen, y el token viaja en lo que la URL llama «ruta».
 *
 * Recortar no pierde lo que sirve: para depurar un bloqueo de CSP lo que
 * importa es qué esquema se bloqueó, no qué había adentro.
 */
const ESQUEMAS_CON_RUTA = new Set([
	'http:',
	'https:',
	'ws:',
	'wss:',
	'ftp:',
	'file:',
	'blob:',
	// Los que usa Tauri para servir la aplicación y hablar con el backend.
	'tauri:',
	'asset:',
	'ipc:',
]);

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
			return `//${url.host}${url.pathname}`;
		} catch {
			// Ni con esquema prestado: no es una URL. Cae al corte de abajo.
		}
	}

	try {
		const url = new URL(valor);
		if (!ESQUEMAS_CON_RUTA.has(url.protocol)) {
			return `${url.protocol}(recortado)`;
		}
		return sinCredenciales(url).href;
	} catch {
		// Una ruta relativa. No puede llevar credenciales —eso necesita una
		// autoridad— así que alcanza con quitarle la query y el fragmento.
		return valor.split(/[?#]/)[0];
	}
}
