import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import type { TrayItem } from '@/interfaces/tray';
import {
	countLabel,
	iconSource,
	itemName,
	launcherForApp,
	mainIcon,
	needsAttention,
	normaliseAppId,
	overlayIcon,
	progressPercent,
	tooltipText,
} from '@/tools/tray-item';

const item = (extra: Partial<TrayItem> = {}): TrayItem => ({
	id: 'discord_status_icon_1',
	service_name: ':1.101',
	status: 'Active',
	category: 'ApplicationStatus',
	...extra,
});

describe('el icono del elemento', () => {
	test('el mapa de bits gana al nombre, y sin ninguno no hay icono', () => {
		expect(iconSource({ name: 'x', data: 'AAA' })).toEqual({ kind: 'pixmap', data: 'AAA' });
		expect(iconSource({ name: 'x' })).toEqual({ kind: 'theme', name: 'x' });
		expect(iconSource({})).toBeNull();
		expect(iconSource(undefined)).toBeNull();
		expect(mainIcon(item())).toBeNull();
	});

	test('con NeedsAttention va el icono de atención, si lo mandó', () => {
		const attention = item({
			status: 'NeedsAttention',
			icon_name: 'telegram-panel',
			attention_icon: { name: 'telegram-attention-panel' },
		});
		expect(mainIcon(attention)).toEqual({ kind: 'theme', name: 'telegram-attention-panel' });
		// Sin NeedsAttention el de atención no se usa aunque esté.
		expect(mainIcon({ ...attention, status: 'Active' })).toEqual({
			kind: 'theme',
			name: 'telegram-panel',
		});
	});

	test('AttentionMovieName sirve si es un nombre, no si es una ruta', () => {
		const movie = item({
			status: 'NeedsAttention',
			icon_name: 'a',
			attention_movie_name: 'a-blink',
		});
		expect(mainIcon(movie)).toEqual({ kind: 'theme', name: 'a-blink' });
		expect(mainIcon({ ...movie, attention_movie_name: '/usr/share/a.mng' })).toEqual({
			kind: 'theme',
			name: 'a',
		});
	});

	test('la insignia superpuesta sólo si viene', () => {
		expect(overlayIcon(item())).toBeNull();
		expect(overlayIcon(item({ overlay_icon: { name: 'emblem-important' } }))).toEqual({
			kind: 'theme',
			name: 'emblem-important',
		});
	});
});

describe('atención, globo, contador y progreso', () => {
	test('pide atención por su estado o por urgent', () => {
		expect(needsAttention(item())).toBe(false);
		expect(needsAttention(item({ status: 'NeedsAttention' }))).toBe(true);
		expect(needsAttention(item({ launcher: { urgent: true } }))).toBe(true);
	});

	test('el globo es título y descripción, y sin nada no hay globo', () => {
		expect(tooltipText(item())).toBeUndefined();
		expect(tooltipText(item({ tooltip: { title: 'Discord' } }))).toBe('Discord');
		expect(tooltipText(item({ title: 'Nextcloud', tooltip: { description: '3 archivos' } }))).toBe(
			'Nextcloud\n3 archivos'
		);
	});

	test('el nombre para quien no ve el icono', () => {
		expect(itemName(item({ tooltip: { title: 'Discord' } }))).toBe('Discord');
		expect(itemName(item({ title: 'Steam' }))).toBe('Steam');
		expect(itemName(item())).toBe('discord_status_icon_1');
	});

	test('el contador: nada en cero, negativo o ausente; 99+ arriba de 99', () => {
		expect(countLabel(undefined)).toBeUndefined();
		expect(countLabel(0)).toBeUndefined();
		expect(countLabel(-3)).toBeUndefined();
		expect(countLabel(Number.NaN)).toBeUndefined();
		expect(countLabel(7)).toBe('7');
		expect(countLabel(100)).toBe('99+');
	});

	test('el progreso fuera de rango se recorta y sin progreso no hay barra', () => {
		expect(progressPercent(undefined)).toBeUndefined();
		expect(progressPercent(Number.NaN)).toBeUndefined();
		expect(progressPercent(-0.4)).toBe(0);
		expect(progressPercent(1.7)).toBe(100);
		expect(progressPercent(0.426)).toBe(43);
	});
});

describe('LauncherEntry sobre las ventanas', () => {
	const entries = [
		{ desktop_id: 'org.telegram.desktop', count: 4 },
		{ desktop_id: 'firefox', progress: 0.5 },
	];

	test('se busca por el app-id normalizado', () => {
		expect(normaliseAppId('org.telegram.desktop')).toBe('telegramdesktop');
		expect(normaliseAppId('firefox')).toBe('firefox');
		expect(normaliseAppId('com.discordapp.Discord')).toBe('discordappdiscord');
		expect(launcherForApp(entries, 'org.telegram.desktop')).toEqual({ count: 4 });
		expect(launcherForApp(entries, 'firefox')).toEqual({ progress: 0.5 });
	});

	test('una ventana sin entrada o sin app-id no recibe nada', () => {
		expect(launcherForApp(entries, 'kitty')).toBeUndefined();
		expect(launcherForApp(entries, undefined)).toBeUndefined();
		expect(launcherForApp([], 'firefox')).toBeUndefined();
	});
});

describe('lo que no viene no se dibuja', () => {
	const root = join(import.meta.dir, '..', '..');
	const read = (path: string) => readFileSync(join(root, path), 'utf8');
	const panel = read('src/components/areas/panel/TrayBarArea.vue');
	const button = read('src/components/buttons/TrayItemButton.vue');
	const windowButton = read('src/components/buttons/WindowPanelButton.vue');

	test('el contador, el punto de atención y la barra van con su v-if', () => {
		expect(panel).toContain('v-if="countLabel(item.launcher?.count)"');
		expect(panel).toContain('v-else-if="needsAttention(item)"');
		expect(panel).toContain('v-if="progressPercent(item.launcher?.progress) !== undefined"');
	});

	test('el globo nativo no queda vacío', () => {
		expect(panel).toContain(':title="tooltipText(item)"');
	});

	test('la insignia superpuesta va con su v-if', () => {
		expect(button).toContain('v-if="overlay"');
	});

	test('el contador y el progreso de las ventanas son los de la librería', () => {
		// Los de `TrayIconButton` 2.2.0: salen de la librería, no se dibujan a
		// mano. Sin progreso le llega `null`, y la librería no dibuja la línea.
		expect(windowButton).toContain(':badge="badge"');
		expect(windowButton).toContain(':progress="progress ?? null"');
		expect(windowButton).not.toContain('<ProgressBar');
	});

	test('la barra de la bandeja es la fina de la librería', () => {
		expect(panel).toMatch(/<ProgressBar[^>]*size="xs"/);
	});

	test('el contador de la bandeja no se parte', () => {
		// `Badge` de la librería parte «99+» en columna sobre un icono de
		// 28 px (ver TrayCountBadge.vue): hasta que se arregle allá, el local
		// no se corta y no se envuelve.
		const badge = read('src/components/indicators/TrayCountBadge.vue');
		const template = badge.slice(badge.indexOf('<template>'));
		expect(template).toContain('whitespace-nowrap');
		expect(template).not.toMatch(/break-words|max-w-full/);
	});
});
