import { describe, expect, test } from 'bun:test';
import {
	DEFAULT_PANEL_ANIMATION,
	DEFAULT_PANEL_LAYOUT,
	DEFAULT_PANEL_SIZE,
	DEFAULT_PANEL_STYLE,
	MAX_PANEL_SIZE,
	MIN_PANEL_SIZE,
	PANEL_ANIMATION_CLASS,
	PANEL_ANIMATIONS,
	PANEL_LAYOUTS,
	PANEL_STYLES,
	panelAnimation,
	panelBarClasses,
	panelLayout,
	panelScaleStyle,
	panelSize,
	panelStyle,
	panelSurfaceClass,
} from './panel-appearance';
import { BAR_CLASSES, isVertical, PANEL_POSITIONS } from './panel-position';

describe('los lectores toleran lo que venga del archivo', () => {
	test('sin nada puesto, los valores de fábrica', () => {
		// La sección `panel` existe desde los indicadores; el caso normal de una
		// instalación vieja es que esté y estas claves no.
		for (const config of [{}, null, { panel: {} }, { panel: { weather: false } }]) {
			expect(panelStyle(config)).toBe(DEFAULT_PANEL_STYLE);
			expect(panelLayout(config)).toBe(DEFAULT_PANEL_LAYOUT);
			expect(panelAnimation(config)).toBe(DEFAULT_PANEL_ANIMATION);
			expect(panelSize(config)).toBe(DEFAULT_PANEL_SIZE);
		}
	});

	test('cada valor conocido se lee', () => {
		for (const style of PANEL_STYLES) expect(panelStyle({ panel: { style } })).toBe(style);
		for (const layout of PANEL_LAYOUTS) expect(panelLayout({ panel: { layout } })).toBe(layout);
		for (const animation of PANEL_ANIMATIONS)
			expect(panelAnimation({ panel: { animation } })).toBe(animation);
	});

	test('cualquier otra cosa cae al valor de fábrica', () => {
		// El archivo se edita a mano: un valor mal escrito no puede dejar el panel
		// sin dibujar.
		expect(panelStyle({ panel: { style: 'isla' } })).toBe(DEFAULT_PANEL_STYLE);
		expect(panelStyle({ panel: { style: 3 } })).toBe(DEFAULT_PANEL_STYLE);
		expect(panelLayout({ panel: { layout: 'centrado' } })).toBe(DEFAULT_PANEL_LAYOUT);
		expect(panelAnimation({ panel: { animation: 'rayo' } })).toBe(DEFAULT_PANEL_ANIMATION);
		expect(panelStyle({ panel: 'bar' })).toBe(DEFAULT_PANEL_STYLE);
	});
});

describe('el tamaño se acota a lo que entra en la franja', () => {
	test('un número dentro del rango se respeta', () => {
		expect(panelSize({ panel: { size: 90 } })).toBe(90);
		expect(panelSize({ panel: { size: 110 } })).toBe(110);
	});

	test('fuera del rango se recorta; lo que no es número vale 100', () => {
		expect(panelSize({ panel: { size: 500 } })).toBe(MAX_PANEL_SIZE);
		expect(panelSize({ panel: { size: 10 } })).toBe(MIN_PANEL_SIZE);
		expect(panelSize({ panel: { size: Number.NaN } })).toBe(DEFAULT_PANEL_SIZE);
		expect(panelSize({ panel: { size: '120' } })).toBe(DEFAULT_PANEL_SIZE);
	});

	test('el factor sale en centésimas para la variable CSS', () => {
		expect(panelScaleStyle(100)).toEqual({ '--panel-scale': '1' });
		expect(panelScaleStyle(80)).toEqual({ '--panel-scale': '0.8' });
	});
});

describe('las clases de la barra por tipo y densidad', () => {
	test('píldoras distribuido es el panel de siempre, al byte', () => {
		// La regresión que más importa: el default no cambia nada de lo que #151
		// dejó. Si cambia, cambia para todas las instalaciones que nunca abrieron
		// esta pantalla. El `relative` que se le antepone vivía antes en la clase
		// fija de la `<nav>`, así que el conjunto de clases del elemento no cambia.
		for (const position of PANEL_POSITIONS) {
			expect(panelBarClasses(position, 'pills', 'distributed')).toBe(
				`relative ${BAR_CLASSES[position]}`
			);
		}
	});

	test('la barra cruza de borde a borde: sin margen y sin encoger', () => {
		for (const position of PANEL_POSITIONS) {
			const classes = panelBarClasses(position, 'bar', 'distributed').split(' ');
			const long = isVertical(position) ? 'h-screen' : 'w-full';
			expect(classes).toContain(long);
			// Ni margen contra el borde ni contra los costados.
			expect(classes.some((name) => /^m[tblrxy]?-/.test(name))).toBe(false);
		}
	});

	test('flotante se despega de los cuatro lados', () => {
		for (const position of PANEL_POSITIONS) {
			const classes = panelBarClasses(position, 'floating', 'distributed').split(' ');
			if (isVertical(position)) {
				expect(classes).toContain('my-1');
				expect(classes).toContain('h-[calc(100vh-8px)]');
			} else {
				expect(classes).toContain('mx-1');
				expect(classes).toContain('w-[calc(100%-8px)]');
			}
		}
	});

	test('compacto se encoge a su contenido y se centra sobre el eje largo', () => {
		// El ancla contra el borde dominante es una utilidad de posición entera
		// (`top-`, `bottom-`, `left-`, `right-`), no `t-`/`b-`: sin ella la barra
		// de abajo compacta volvía al borde de arriba.
		const anchor: Record<string, string> = {
			top: 'top-',
			bottom: 'bottom-',
			left: 'left-',
			right: 'right-',
		};
		for (const position of PANEL_POSITIONS) {
			for (const style of PANEL_STYLES) {
				const classes = panelBarClasses(position, style, 'compact').split(' ');
				expect(classes).toContain('absolute');
				expect(classes.some((name) => name.startsWith(anchor[position] as string))).toBe(true);
				const track = isVertical(position) ? 'grid-rows' : 'grid-cols';
				expect(classes).toContain(`${track}-[auto_auto_auto]`);
				if (isVertical(position)) {
					expect(classes).toContain('h-fit');
					expect(classes).toContain('-translate-y-1/2');
				} else {
					expect(classes).toContain('w-fit');
					expect(classes).toContain('-translate-x-1/2');
				}
			}
		}
	});
});

describe('la superficie detrás de las píldoras', () => {
	test('las píldoras no tienen superficie', () => {
		for (const position of PANEL_POSITIONS) {
			expect(panelSurfaceClass('pills', position)).toBe('');
		}
	});

	test('flotante, barra y dock son fondos translúcidos', () => {
		for (const style of ['floating', 'bar', 'dock'] as const) {
			for (const position of PANEL_POSITIONS) {
				expect(panelSurfaceClass(style, position)).toContain('bg-ui-bg/80');
			}
		}
	});

	test('la barra no redondea; el dock redondea sólo el lado de adentro', () => {
		expect(panelSurfaceClass('bar', 'top')).not.toContain('rounded');
		expect(panelSurfaceClass('floating', 'top')).toContain('rounded-corner-m');
		expect(panelSurfaceClass('dock', 'top')).toContain('rounded-b-corner-m');
		expect(panelSurfaceClass('dock', 'bottom')).toContain('rounded-t-corner-m');
		expect(panelSurfaceClass('dock', 'left')).toContain('rounded-r-corner-m');
		expect(panelSurfaceClass('dock', 'right')).toContain('rounded-l-corner-m');
	});
});

describe('las animaciones', () => {
	test('apagada no pone ninguna clase; cada otra tiene la suya', () => {
		expect(PANEL_ANIMATION_CLASS.off).toBe('');
		for (const animation of PANEL_ANIMATIONS) {
			if (animation === 'off') continue;
			expect(PANEL_ANIMATION_CLASS[animation]).toBe(`panel-anim-${animation}`);
		}
	});

	test('cada clase que se usa está declarada en main.css', () => {
		const css = Bun.file(new URL('../assets/main.css', import.meta.url)).text();
		return css.then((text) => {
			for (const animation of PANEL_ANIMATIONS) {
				if (animation === 'off') continue;
				expect(text).toContain(`.${PANEL_ANIMATION_CLASS[animation]}`);
			}
		});
	});
});
