import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Las superficies que dejó pendientes #144 y que pasan a los componentes de
 * vue-libvasak 2.2.0 (vue-libvasak#74, inventario §1). Lo que se mira acá es
 * que cada pantalla **pida** su pieza a la librería y que no vuelva la copia a
 * mano; la conducta de cada pieza (el teclado, el ARIA, los estados) se prueba
 * montada en la librería, que es donde vive.
 */
/** Saca los comentarios HTML cortando por sus delimitadores, sin expresiones regulares. */
function stripHtmlComments(text: string): string {
	let out = '';
	let index = 0;
	while (index < text.length) {
		const start = text.indexOf('<!--', index);
		if (start === -1) return out + text.slice(index);
		out += text.slice(index, start);
		const end = text.indexOf('-->', start + 4);
		if (end === -1) return out;
		index = end + 3;
	}
	return out;
}

const ROOT = join(import.meta.dir, '..', 'src');
const read = (file: string) => readFileSync(join(ROOT, file), 'utf8');
const template = (file: string) => {
	const text = read(file);
	// Sin comentarios: lo que se explica no es lo que se dibuja.
	return stripHtmlComments(text.slice(text.indexOf('<template>'), text.lastIndexOf('</template>')));
};

describe('cada superficie pide sus piezas a la librería', () => {
	test.each([
		['views/apps/OsdPopupView.vue', ['<ProgressBar']],
		['views/apps/SessionPopupView.vue', ['<ActionButton']],
		['views/ConnectMenuView.vue', ['<SegmentedControl', '<AlertMessage', '<EmptyState', '<LoadingState', '<SettingRow', '<ActionButton']],
		['components/buttons/ConnectAppButton.vue', ['<ListRow']],
		['components/cards/PhoneControlCenterCard.vue', ['<Panel', '<ListRow', '<StatusDot', '<SwitchToggle', '<ActionButton']],
		['components/cards/NotificationGroupCard.vue', ['<Disclosure', '<Badge', '<ActionButton']],
		['components/cards/NotificationCard.vue', ['<ActionButton']],
		['components/areas/control-center/NotificationArea.vue', ['<ActionButton', '<EmptyState']],
		['components/cards/UserControlCenterCard.vue', ['<Avatar']],
		['views/applets/PrivacyAppletView.vue', ['<ListRow', '<ActionButton']],
		['views/applets/TwingateAppletView.vue', ['<ListRow', '<StatusDot', '<Badge', '<ActionButton']],
		['components/controls/AudioDeviceSelector.vue', ['<OptionGroup', '<LoadingState']],
		['views/applets/MusicAppletView.vue', ['<OptionGroup', '<Chip', '<PageDots', '<ActionButton']],
		['components/controls/TrayMusicControl.vue', ['<SpinningCover', '<PanelPill']],
	])('%s', (file, pieces) => {
		const view = template(file);
		for (const piece of pieces) expect(view).toContain(piece);
		// Ni un `<button>` propio: los botones son `ActionButton`, y lo que se
		// despliega o se elige es de la librería.
		expect(view).not.toMatch(/<button\b/);
	});
});

describe('lo que dibujaban a mano no vuelve', () => {
	test('el OSD ya no arma la barra con un ancho en línea', () => {
		const view = template('views/apps/OsdPopupView.vue');
		expect(view).toMatch(/<ProgressBar[\s\S]*?size="xs"/);
		expect(view).not.toContain(':style');
	});

	test('la sesión: el aro de carga es el `loading` del botón', () => {
		const view = template('views/apps/SessionPopupView.vue');
		expect(view).toContain(':loading="confirming"');
		expect(view).not.toContain('animate-spin');
		// El texto atenuado es `tx-muted`, no el principal con opacidad.
		expect(view).not.toMatch(/text-tx-main\/\d/);
		// Y la caja se achica si la ventana es angosta en vez de desbordar.
		expect(view).toContain('max-w-[380px]');
		expect(view).not.toMatch(/(?<![\w-])w-\[380px\]/);
	});

	test('ningún tamaño de letra en píxeles en lo que se migró', () => {
		for (const file of [
			'components/cards/NotificationGroupCard.vue',
			'components/cards/NotificationCard.vue',
			'views/applets/TwingateAppletView.vue',
		]) {
			expect(template(file)).not.toMatch(/text-\[\d+px\]/);
		}
	});

	test('las curvas de las notificaciones son las del sistema', () => {
		for (const file of ['components/areas/control-center/NotificationArea.vue', 'components/cards/NotificationGroupCard.vue']) {
			expect(template(file)).not.toContain('cubic-bezier');
		}
	});
});

describe('la salida de audio la marca el sistema, no el clic', () => {
	test.each([
		['components/controls/AudioDeviceSelector.vue', 'checkedDevice'],
		['views/applets/MusicAppletView.vue', 'checkedOutput'],
	])('%s', (file, model) => {
		// `OptionGroup` mueve su marca antes de avisar: sin un `v-model` que
		// mande, una salida que no se pudo poner quedaba elegida. El setter
		// vacío deja que la marca la ponga lo que confirmó el sistema.
		const text = read(file);
		expect(template(file)).toContain(`v-model="${model}"`);
		const declaration = text.slice(text.indexOf(`const ${model} = computed`));
		expect(declaration.length).toBeGreaterThan(0);
		expect(declaration.slice(0, declaration.indexOf('});'))).toContain('set: () => {}');
	});
});

describe('el marco de los widgets', () => {
	test('es uno solo, propio del escritorio, y lo usan los dos lugares', () => {
		expect(template('components/widgets/WidgetHost.vue')).toContain('<WidgetFrame');
		expect(template('components/widgets/WidgetSlot.vue')).toContain('<WidgetFrame surface="surface"');
		// La caja contra la que se miden los widgets se declara en el marco y
		// en ningún otro lado.
		expect(template('components/widgets/WidgetHost.vue')).not.toContain('container-type');
		expect(template('components/widgets/WidgetSlot.vue')).not.toContain('container-type');
		expect(template('components/widgets/WidgetFrame.vue')).toContain('container-type: size');
	});

	test('las dos superficies salen del esquema y son translúcidas; sólo la del escritorio desenfoca', () => {
		const frame = template('components/widgets/WidgetFrame.vue');
		// Sobre el fondo de pantalla, Wayfire no tiene nada detrás que
		// desenfocar: el desenfoque lo pone el marco (decisión del 02/10/2026).
		expect(frame).toContain("'bg-ui-shell shadow-surface-m backdrop-blur-md'");
		expect(frame).not.toContain("'bg-ui-float shadow-surface-m");
		// El suelto del menú va dentro de una ventana que ya desenfoca Wayfire.
		expect(frame).toContain(": 'bg-ui-surface/70'");
		expect(frame.match(/backdrop-blur/g)).toHaveLength(1);
	});

	test('en edición, el contenido no recibe clics y el tirador avisa', () => {
		const frame = template('components/widgets/WidgetFrame.vue');
		expect(frame).toContain("editing ? 'pointer-events-none select-none' : ''");
		expect(frame).toContain("@pointerdown=\"emit('resize-start', $event)\"");
		expect(template('components/widgets/WidgetHost.vue')).toContain('@resize-start="startResize"');
	});
});
