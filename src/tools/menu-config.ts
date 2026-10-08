/**
 * La configuración del menú de inicio (vasak-desktop#203).
 *
 * Todo sale de la sección `menu` de `~/.config/vasak/vasak.conf`, la misma que
 * edita la pantalla «Menú de inicio» de Configuración. Se lee **tolerante**, con
 * la misma regla que el panel (`panel-appearance.ts`): el archivo se edita a
 * mano, así que un valor que no sea de los que se conocen —o que no diga nada—
 * cae al de fábrica y nunca deja el menú sin dibujar. Nada se castea: si viene
 * algo raro, se ignora.
 *
 * El contrato de las claves está en `menu-variants-spec.md` y tiene que coincidir
 * con lo que escribe `vasak-settings`. Si cambiás el esquema, cambialo ahí
 * primero.
 *
 * Vive aparte de las vistas para poder probarlo sin montar Vue.
 */

// ── Variante (esqueleto del menú) ────────────────────────────────────────────

/** Los cinco esqueletos del menú. `compact` es el de siempre. */
export const MENU_VARIANTS = ['compact', 'classic', 'grid', 'favorites', 'tiles'] as const;

export type MenuVariant = (typeof MENU_VARIANTS)[number];

/** El de siempre: la distribución de tres zonas de vasak-desktop#74. */
export const DEFAULT_MENU_VARIANT: MenuVariant = 'compact';

export function isMenuVariant(value: unknown): value is MenuVariant {
	return typeof value === 'string' && (MENU_VARIANTS as readonly string[]).includes(value);
}

// ── Widget del hueco ─────────────────────────────────────────────────────────

/**
 * Qué widget va en el hueco del menú (en las variantes que lo tengan).
 *
 * Son los del escritorio (`tools/widgets/catalog.ts`) más `'none'`, que apaga el
 * hueco. El `WidgetSlot` sólo acepta los primeros cuatro, así que `'none'` se
 * mira aparte antes de pasárselo.
 */
export const MENU_WIDGETS = ['clock', 'music', 'weather', 'files', 'none'] as const;

export type MenuWidget = (typeof MENU_WIDGETS)[number];

/** El clima, que es lo que había fijo. */
export const DEFAULT_MENU_WIDGET: MenuWidget = 'weather';

export function isMenuWidget(value: unknown): value is MenuWidget {
	return typeof value === 'string' && (MENU_WIDGETS as readonly string[]).includes(value);
}

// ── Posición del buscador ────────────────────────────────────────────────────

export const SEARCH_POSITIONS = ['top', 'bottom'] as const;

export type SearchPosition = (typeof SEARCH_POSITIONS)[number];

/** Arriba, que es donde estuvo siempre. */
export const DEFAULT_SEARCH_POSITION: SearchPosition = 'top';

export function isSearchPosition(value: unknown): value is SearchPosition {
	return typeof value === 'string' && (SEARCH_POSITIONS as readonly string[]).includes(value);
}

// ── Encabezado ───────────────────────────────────────────────────────────────

export const MENU_HEADERS = ['none', 'hero'] as const;

export type MenuHeader = (typeof MENU_HEADERS)[number];

/** Sin encabezado, como el menú de hoy. */
export const DEFAULT_MENU_HEADER: MenuHeader = 'none';

export function isMenuHeader(value: unknown): value is MenuHeader {
	return typeof value === 'string' && (MENU_HEADERS as readonly string[]).includes(value);
}

/** Visibilidad de la imagen del hero, de 0 a 100. */
export const DEFAULT_HEADER_STRENGTH = 60;

// ── La forma resuelta ────────────────────────────────────────────────────────

export interface MenuConfig {
	variant: MenuVariant;
	widget: MenuWidget;
	showUser: boolean;
	showSessionActions: boolean;
	searchPosition: SearchPosition;
	showPlaces: boolean;
	/** Rutas `.desktop` fijadas como favoritas. */
	favorites: string[];
	showFavorites: boolean;
	header: MenuHeader;
	/** Ruta de imagen del hero; vacío es sin imagen. */
	headerImage: string;
	/** Opacidad de la imagen del hero, de 0 a 100. */
	headerStrength: number;
	showGreeting: boolean;
	showWeather: boolean;
}

/**
 * La sección `menu` cruda del archivo (lo que está escrito, sin valores de
 * fábrica), o un objeto vacío si no hay nada. Se usa para mezclar sin perder las
 * claves que esta versión no conoce.
 */
function rawMenuSection(config: unknown): Record<string, unknown> {
	const section =
		config && typeof config === 'object' ? (config as Record<string, unknown>).menu : undefined;
	return section && typeof section === 'object' ? (section as Record<string, unknown>) : {};
}

/**
 * Mezcla `partial` sobre la sección `menu` cruda, preservando las claves ajenas.
 *
 * Parte de lo que está escrito —no de lo resuelto— para no guardar de vuelta
 * todos los valores de fábrica como si la persona los hubiera elegido, y deja
 * intacto lo que esta versión no conoce (la regla `{ ...config.menu, ...nuevo }`
 * del taller).
 */
export function mergeMenuSection(
	config: unknown,
	partial: Partial<MenuConfig>
): Record<string, unknown> {
	return { ...rawMenuSection(config), ...partial };
}

/** El token del archivo para una clave de la sección `menu`. */
function menuKey(config: unknown, key: string): unknown {
	const section =
		config && typeof config === 'object'
			? ((config as Record<string, unknown>).menu as Record<string, unknown> | undefined)
			: undefined;
	return section?.[key];
}

/**
 * Un booleano tolerante: sólo un `true`/`false` de verdad cuenta; cualquier otra
 * cosa —ausente, un string, un número— cae al de fábrica. Así una clave mal
 * escrita a mano no apaga una parte del menú sin querer.
 */
function readBoolean(value: unknown, fallback: boolean): boolean {
	return typeof value === 'boolean' ? value : fallback;
}

/** Un número acotado a [min, max]; cualquier cosa que no sea finita cae al de fábrica. */
function readStrength(value: unknown, fallback: number): number {
	if (typeof value !== 'number' || !Number.isFinite(value)) return fallback;
	return Math.min(100, Math.max(0, value));
}

/** Un string limpio, o el de fábrica si no es un string. */
function readString(value: unknown, fallback: string): string {
	return typeof value === 'string' ? value : fallback;
}

/**
 * La lista de favoritos: sólo las entradas que son strings no vacíos, sin
 * repetir y en orden. Lo que no sea un arreglo da una lista vacía.
 */
export function readFavorites(value: unknown): string[] {
	if (!Array.isArray(value)) return [];
	const seen = new Set<string>();
	const result: string[] = [];
	for (const entry of value) {
		if (typeof entry !== 'string') continue;
		const path = entry.trim();
		if (!path || seen.has(path)) continue;
		seen.add(path);
		result.push(path);
	}
	return result;
}

/**
 * Lee la sección `menu` entera, con todos los valores de fábrica puestos.
 *
 * Es lo que consumen las vistas: una forma completa y segura, nunca `undefined`.
 */
export function readMenuConfig(config: unknown): MenuConfig {
	const variant = menuKey(config, 'variant');
	const widget = menuKey(config, 'widget');
	const searchPosition = menuKey(config, 'searchPosition');
	const header = menuKey(config, 'header');

	return {
		variant: isMenuVariant(variant) ? variant : DEFAULT_MENU_VARIANT,
		widget: isMenuWidget(widget) ? widget : DEFAULT_MENU_WIDGET,
		showUser: readBoolean(menuKey(config, 'showUser'), true),
		showSessionActions: readBoolean(menuKey(config, 'showSessionActions'), true),
		searchPosition: isSearchPosition(searchPosition) ? searchPosition : DEFAULT_SEARCH_POSITION,
		showPlaces: readBoolean(menuKey(config, 'showPlaces'), false),
		favorites: readFavorites(menuKey(config, 'favorites')),
		showFavorites: readBoolean(menuKey(config, 'showFavorites'), false),
		header: isMenuHeader(header) ? header : DEFAULT_MENU_HEADER,
		headerImage: readString(menuKey(config, 'headerImage'), ''),
		headerStrength: readStrength(menuKey(config, 'headerStrength'), DEFAULT_HEADER_STRENGTH),
		showGreeting: readBoolean(menuKey(config, 'showGreeting'), true),
		showWeather: readBoolean(menuKey(config, 'showWeather'), true),
	};
}
