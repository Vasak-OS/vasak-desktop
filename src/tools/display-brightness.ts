/**
 * Lo que el centro de control decide sobre el brillo de las pantallas
 * (vasak-desktop#189), sin Vue ni Tauri: cuál es el monitor principal, cómo se
 * llama cada uno, qué decir de los monitores externos y cómo escribir sólo el
 * último valor.
 */
import type {
	BrightnessReport,
	DdcStatus,
	MonitorBrightness,
} from '@vasakgroup/plugin-display-manager';

/** La identidad de un monitor dentro del informe: su vía y su manija. */
export function monitorKey(monitor: Pick<MonitorBrightness, 'kind' | 'handle'>): string {
	return `${monitor.kind}:${monitor.handle}`;
}

/**
 * El monitor del estado A: el panel interno si hay uno —el que regulan las
 * teclas de brillo— y si no, el primer monitor externo que contesta DDC/CI.
 */
export function primaryMonitor(report: BrightnessReport | null): MonitorBrightness | null {
	if (!report) return null;
	return report.monitors.find((m) => m.kind === 'backlight') ?? report.monitors[0] ?? null;
}

/**
 * El nombre que se ve: «Integrada» para el panel interno y el conector para
 * los externos (HDMI-A-1). Con más de un panel interno se agrega el conector,
 * para que se distingan.
 */
export function monitorName(
	monitor: MonitorBrightness,
	monitors: readonly MonitorBrightness[],
	builtIn: string
): string {
	if (monitor.kind === 'backlight') {
		const panels = monitors.filter((m) => m.kind === 'backlight').length;
		return panels > 1 && monitor.output ? `${builtIn} (${monitor.output})` : builtIn;
	}
	return monitor.output ?? monitor.handle;
}

/** Un aviso para mostrar: la clave de traducción y sus argumentos (`{0}`). */
export interface DdcNotice {
	key: string;
	args: string[];
	/** Si dice que algo no está disponible (y no que se está buscando). */
	unavailable: boolean;
}

const REASONS: Record<NonNullable<DdcStatus['reason']>, string> = {
	'not-installed': 'components.MonitorBrightness.ddcNotInstalled',
	'no-i2c-dev': 'components.MonitorBrightness.ddcNoI2cDev',
	'no-permission': 'components.MonitorBrightness.ddcNoPermission',
};

/**
 * Qué decir de los monitores externos. El plugin manda códigos y no frases;
 * el texto sale traducido. Un monitor que no contesta DDC/CI se nombra como no
 * disponible, en vez de desaparecer de la lista sin explicación. Sin monitores
 * externos no se dice nada: una notebook sola no necesita ddcutil.
 */
export function ddcNotices(status: DdcStatus): DdcNotice[] {
	if (status.state === 'detecting') {
		return [{ key: 'components.MonitorBrightness.ddcDetecting', args: [], unavailable: false }];
	}
	if (status.state === 'unavailable' && status.reason) {
		return [{ key: REASONS[status.reason], args: [], unavailable: true }];
	}
	return status.unsupported.map((output) => ({
		key: 'components.MonitorBrightness.ddcUnsupported',
		args: [output],
		unavailable: true,
	}));
}

/**
 * Escribe sólo el último valor pedido para cada clave.
 *
 * Mientras una escritura está en curso, lo que llega después se guarda —y
 * pisa lo guardado— y se escribe al terminar. Un monitor por DDC/CI tarda
 * 40 ms o más por escritura: en fila, seguiría cambiando segundos después de
 * soltar. Cada clave es independiente: mover un monitor no espera al otro.
 *
 * Devuelve una promesa que se cumple cuando la clave quedó con su último
 * valor escrito (o falló).
 */
export function createLatestWriter<V>(write: (key: string, value: V) => Promise<void>) {
	const pending = new Map<string, V>();
	const running = new Map<string, Promise<void>>();

	/**
	 * Una escritura que falla no se lleva puesta a la que esperaba detrás; lo
	 * que cuenta es cómo terminó la última.
	 */
	async function drain(key: string): Promise<void> {
		let failure: unknown = null;
		while (pending.has(key)) {
			const value = pending.get(key) as V;
			pending.delete(key);
			try {
				await write(key, value);
				failure = null;
			} catch (error) {
				failure = error;
			}
		}
		if (failure !== null) throw failure;
	}

	return function request(key: string, value: V): Promise<void> {
		pending.set(key, value);
		const current = running.get(key);
		if (current) return current;
		const task = drain(key).finally(() => {
			running.delete(key);
		});
		running.set(key, task);
		return task;
	};
}
