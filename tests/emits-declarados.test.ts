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
 * nada avise.
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

	test('SessionButton declara `click`', () => {
		const fuente = leer('src/components/buttons/SessionButton.vue');

		expect(fuente).toMatch(/defineEmits<\{\s*click:/);
	});

	test('y lo cablea en el botón, o los cuatro botones quedan mudos', () => {
		const fuente = leer('src/components/buttons/SessionButton.vue');

		expect(fuente).toMatch(/<button[^>]*@click="emit\('click', \$event\)"/s);
	});
});
