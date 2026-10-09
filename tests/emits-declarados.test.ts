/**
 * Dos componentes que recibían su evento por caída de atributos.
 *
 * Cuando un componente **no** declara `defineEmits`, un `@click` o un `@action`
 * puesto desde afuera no es un evento suyo: queda en `attrs` y cae sobre el
 * nodo raíz que dibuje. A veces funciona y a veces no, y en los dos casos de
 * acá pasaban las dos cosas. Lo destapó `strictTemplates`.
 *
 * **`BluetoothDeviceCard`** re-emitía con `$emit('action')` sin declarar el
 * emit, y su raíz es un `DeviceCard` que **sí** declara `action`. Así que el
 * manejador del padre llegaba por dos caminos —el re-emitido y el caído— y
 * corría **dos veces** por clic: conectar y desconectar un dispositivo se
 * pedían por duplicado. Comprobado montando el patrón con `@vue/test-utils`:
 * sin declarar el emit, dos llamadas; declarándolo, una.
 *
 * **`SessionButton`** recibía el `@click` por caída sobre su `<button>`, que
 * funcionaba. Pero al declarar el emit esa caída se corta, así que había que
 * cablearlo a mano en la plantilla: declararlo sin reemitir deja el botón mudo,
 * y apagar, reiniciar, cerrar sesión y suspender dejan de responder sin que
 * nada avise. Ya no existe: los botones de sesión del menú son el
 * `ActionButton` de la librería, que declara `click` y lo prueba allá; lo que
 * se vigila acá es que el menú siga escuchándolo.
 *
 * Se fija por el texto del componente y no montándolo porque este repositorio
 * no tiene con qué montar; lo que se vigila es que la declaración y el cable
 * sigan juntos, que es el par que se rompe.
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

const raiz = join(import.meta.dir, '..');
const leer = (ruta: string) => readFileSync(join(raiz, ruta), 'utf8');

describe('los eventos van declarados', () => {
	test('BluetoothDeviceCard declara `action`', () => {
		const fuente = leer('src/components/cards/BluetoothDeviceCard.vue');

		expect(fuente).toMatch(/defineEmits<\{\s*action:/);
	});

	test('y sigue re-emitiéndolo, que es lo que lo hace llegar', () => {
		// Declarar el emit corta la caída de atributos: sin el `$emit`, el
		// padre deja de enterarse y conectar un dispositivo no hace nada.
		const fuente = leer('src/components/cards/BluetoothDeviceCard.vue');

		expect(fuente).toMatch(/@action="\$emit\('action'\)"/);
	});

	test('los botones de sesión del menú escuchan el `click` del botón de la librería', () => {
		// Los botones se mudaron a `MenuSessionActions`, que comparten todas las
		// variantes del menú (vasak-desktop#203). Si dejara de pasar el
		// manejador, los cinco botones quedarían mudos sin que nada avise.
		const fuente = leer('src/components/areas/menu/MenuSessionActions.vue');
		const boton = fuente.slice(fuente.indexOf('<ActionButton'), fuente.indexOf('/>', fuente.indexOf('<ActionButton')));

		expect(boton).toMatch(/v-for="action in actions"/);
		expect(boton).toMatch(/@click="action\.handler"/);
		expect(fuente).not.toMatch(/SessionButton/);
		// Y la vista le pasa las acciones del controlador.
		expect(leer('src/views/MenuView.vue')).toMatch(/<MenuSessionActions[^>]*:actions="sessionActions"/);
	});
});
