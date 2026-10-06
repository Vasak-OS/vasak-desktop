/**
 * Lo que hacen los botones del centro de control que abren otra cosa: el
 * tablero de tiempo de pantalla y el lanzador.
 *
 * Están acá y no dentro de cada botón porque los usan dos formas del mismo
 * control: el botón redondo y el mosaico del estado B (vasak-desktop#175). Una
 * copia en cada uno sería una copia que se separa.
 */
import { Command } from '@tauri-apps/plugin-shell';
import { hideControlCenter, toggleApplet } from '@/services/window.service';

// Los errores van por `console.error` y no por `logError`: el logger del
// escritorio reemplaza `console.error` al arrancar y lo manda al mismo archivo,
// así que en la sesión es lo mismo. Importar el logger acá lo construiría en
// las pruebas antes que `el-logger-no-se-llama-a-si-mismo.test.ts`, que necesita
// ser el primero en hacerlo (y en CI el orden de los archivos no es fijo).

/**
 * Abre el tablero de tiempo de pantalla (vasak-desktop#150).
 *
 * El tablero es un applet anclado (`screen-time` en `APPLETS`); sin botón del
 * panel del que colgar, el backend lo centra en el eje del panel. Antes de
 * abrirlo se cierra el centro de control: el tablero ocupa el centro de la
 * pantalla, y dos superficies flotando a la vez se pisan.
 */
export async function openScreenTime(): Promise<void> {
	try {
		await hideControlCenter();
		await toggleApplet('screen-time');
	} catch (error) {
		console.error('[control-center] no se pudo abrir el tablero de tiempo de pantalla:', error);
	}
}

/**
 * Abre el lanzador.
 *
 * Abre `vasak-prism`, que es otro programa: la búsqueda global salió del
 * escritorio para poder quedarse residente —que es lo que la hace instantánea—
 * y crecer por su cuenta.
 *
 * `--toggle` y no el binario a secas: sin el argumento, cada clic dejaría un
 * proceso más. Con él, el que arranca le habla por D-Bus al que ya está y se
 * muere; y si no hay ninguno, se queda él. Es exactamente lo que hace el atajo
 * del teclado en `wayfire.ini`.
 *
 * El lanzamiento necesita permiso: `shell:allow-spawn` de
 * `capabilities/default.json` lista los binarios que esta ventana puede lanzar.
 * Sin `vasak-prism` en esa lista lo rechaza el propio Tauri, en tiempo de
 * ejecución y sólo en el paquete instalado.
 */
export async function openSearch(): Promise<void> {
	try {
		await Command.create('vasak-prism', ['--toggle']).spawn();
	} catch (error) {
		console.error('[control-center] no se pudo abrir el lanzador:', error);
	}
}
