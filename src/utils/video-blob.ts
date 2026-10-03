/**
 * Reproducir un video del disco en WebKitGTK, y no más de uno a la vez.
 *
 * # Por qué un blob
 *
 * Un `<video src>` apuntando al protocolo de assets de Tauri no funciona, y no
 * por los codecs: el elemento multimedia de WebKit no se sirve del cargador de
 * recursos de la página sino de GStreamer, que no sabe leer de un esquema
 * propio. El handler entrega los bytes y el video igual termina en error 4
 * (SRC_NOT_SUPPORTED). Lo que funciona es traer los bytes con `fetch` sobre el
 * mismo protocolo y reproducirlos desde un blob. El costo es tenerlos en
 * memoria, así que hay un tope.
 *
 * Lo usan el fondo en movimiento (`DesktopView.vue`) y la previsualización del
 * selector de fondos (`WallpaperPickerView.vue`).
 */

/** Un fondo en bucle son unas decenas de megas; una película no es un fondo. */
export const MAX_VIDEO_BYTES = 128 * 1024 * 1024;

export class VideoTooLargeError extends Error {
	constructor(readonly bytes: number) {
		super(
			`pesa ${Math.round(bytes / 1024 / 1024)} MB y el límite es ${MAX_VIDEO_BYTES / 1024 / 1024} MB`
		);
	}
}

/**
 * Trae el archivo y devuelve una URL `blob:` para el `<video>`. Quien la pide la
 * suelta con `URL.revokeObjectURL` cuando deja de usarla.
 */
export async function fetchVideoBlob(url: string, fetcher: typeof fetch = fetch): Promise<string> {
	const response = await fetcher(url);
	if (!response.ok) throw new Error(`respuesta ${response.status}`);

	const declared = Number(response.headers.get('content-length') ?? 0);
	if (declared > MAX_VIDEO_BYTES) throw new VideoTooLargeError(declared);

	const bytes = await response.blob();
	if (bytes.size > MAX_VIDEO_BYTES) throw new VideoTooLargeError(bytes.size);

	return URL.createObjectURL(bytes);
}

/**
 * Una sola previsualización viva.
 *
 * `show(id)` carga la de ese fondo y **suelta la anterior en el acto**, antes de
 * que llegue la nueva; `show(null)` suelta la que haya. Si una carga termina
 * cuando ya se pidió otra, esa URL se suelta sin usarse: recorrer la fila
 * rápido no deja blobs —ni decodificadores— colgados.
 */
export function createPreviewLoader(options: {
	load: (id: string) => Promise<string>;
	release: (url: string) => void;
	onChange: (id: string | null, url: string | null) => void;
}) {
	let wanted: string | null = null;
	let alive: { id: string; url: string } | null = null;

	function drop(): void {
		if (alive) {
			options.release(alive.url);
			alive = null;
		}
	}

	async function show(id: string | null): Promise<void> {
		if (id === wanted && (alive?.id === id || id === null)) return;
		wanted = id;
		drop();
		options.onChange(id, null);
		if (id === null) return;

		let url: string;
		try {
			url = await options.load(id);
		} catch {
			// Sin previsualización queda la miniatura quieta: no es un error que
			// valga la pena mostrar.
			return;
		}

		if (wanted !== id) {
			options.release(url);
			return;
		}
		alive = { id, url };
		options.onChange(id, url);
	}

	return {
		show,
		/** Suelta todo: al cerrar el selector. */
		clear(): void {
			wanted = null;
			drop();
			options.onChange(null, null);
		},
		/** Cuántas hay vivas: cero o una. Para las pruebas. */
		get aliveCount(): number {
			return alive ? 1 : 0;
		},
	};
}
