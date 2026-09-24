import {
	getLastLogLines,
	getLogFilePath,
	logFromFrontend,
	readLogFile,
} from '@/services/core.service';

/**
 * Niveles de log disponibles
 */
export enum LogLevel {
	DEBUG = 'DEBUG',
	INFO = 'INFO',
	WARNING = 'WARNING',
	ERROR = 'ERROR',
}

/** Las cuatro funciones de consola que este logger reemplaza. */
type ConsoleMethod = 'log' | 'debug' | 'warn' | 'error';

/**
 * Interfaz para el logger de Vasak Desktop
 */
class VasakLogger {
	private isDevelopment: boolean;

	/**
	 * Las funciones de consola tal como estaban antes de que el constructor las
	 * reemplazara. Todo lo que el logger quiera decir sale por acá: decirlo por
	 * la consola reemplazada es volver a entrar al logger, y esa vuelta no
	 * tiene fondo.
	 */
	private readonly originalConsole: Record<ConsoleMethod, (...args: any[]) => void>;

	/**
	 * Verdadero mientras se está entregando un log al backend. Lo que llegue a
	 * la consola dentro de esa ventana viene de adentro del envío —del `invoke`
	 * o de un plugin que lo envuelva—, así que se imprime pero no se reenvía.
	 */
	private forwarding = false;

	constructor() {
		this.isDevelopment = import.meta.env.DEV;
		this.originalConsole = {
			log: console.log.bind(console),
			debug: console.debug.bind(console),
			warn: console.warn.bind(console),
			error: console.error.bind(console),
		};
		this.initializeLogger();
	}

	/**
	 * Inicializa el logger y captura errores globales
	 */
	private initializeLogger() {
		// Capturar errores no manejados
		window.addEventListener('error', (event) => {
			this.error(`Error no manejado: ${event.message}`, {
				filename: event.filename,
				lineno: event.lineno,
				colno: event.colno,
				error: event.error?.stack || event.error?.toString(),
			});
		});

		// Capturar promesas rechazadas no manejadas
		window.addEventListener('unhandledrejection', (event) => {
			this.error(`Promise rechazada no manejada: ${event.reason}`, {
				reason: event.reason,
			});
		});

		// Interceptar console.error y console.warn
		console.error = (...args: any[]) => {
			if (this.isDevelopment) {
				this.originalConsole.error(...args);
			}
			if (this.forwarding) {
				return;
			}
			this.error(this.formatArgs(args));
		};

		console.warn = (...args: any[]) => {
			if (this.isDevelopment) {
				this.originalConsole.warn(...args);
			}
			if (this.forwarding) {
				return;
			}
			this.warning(this.formatArgs(args));
		};

		// console.log and console.debug are only intercepted in development.
		// In production they were shipped to the backend as INFO — one IPC round
		// trip and a disk write per call — for output nobody reads. Errors and
		// warnings are always captured, which is what a log is for.
		if (!this.isDevelopment) {
			return;
		}

		console.log = (...args: any[]) => {
			this.originalConsole.log(...args);
			if (this.forwarding) {
				return;
			}
			this.info(this.formatArgs(args));
		};

		console.debug = (...args: any[]) => {
			this.originalConsole.debug(...args);
			if (this.forwarding) {
				return;
			}
			this.debug(this.formatArgs(args));
		};

		this.info('Sistema de logging inicializado');
	}

	/**
	 * Formatea argumentos de console a string
	 */
	private formatArgs(args: any[]): string {
		return args
			.map((arg) => {
				if (typeof arg === 'object') {
					try {
						return JSON.stringify(arg, null, 2);
					} catch {
						return String(arg);
					}
				}
				return String(arg);
			})
			.join(' ');
	}

	/**
	 * Avisa de un envío que no llegó.
	 *
	 * Sale por la consola original a propósito. La reemplazada vuelve a entrar
	 * al logger, que vuelve a intentar el envío, que vuelve a fallar, que
	 * vuelve a avisar: con el backend caído la pestaña se queda sin memoria
	 * antes de dibujar nada.
	 */
	private reportDeliveryFailure(error: unknown) {
		if (this.isDevelopment) {
			this.originalConsole.warn('[Logger] No se pudo enviar el log al backend:', error);
		}
	}

	/**
	 * Envía un log al backend de Rust
	 */
	private async sendLog(level: LogLevel, message: string, data?: any) {
		const normalizedData =
			data instanceof Error ? { name: data.name, message: data.message, stack: data.stack } : data;
		const fullMessage =
			normalizedData !== undefined
				? `${message} | Data: ${JSON.stringify(normalizedData)}`
				: message;

		let delivery: Promise<unknown>;

		// La bandera cubre la parte síncrona del envío, que es donde la
		// reentrada no tiene fondo: lo que el `invoke` escriba en consola
		// ocurre acá, antes de que el bucle de eventos avance. Cubrir también
		// la espera descartaría logs legítimos que se hayan emitido mientras
		// éste viajaba, que es justo lo que no se quiere perder.
		this.forwarding = true;
		try {
			delivery = logFromFrontend({
				level: level.toString(),
				message: fullMessage,
			});
		} catch (error) {
			this.reportDeliveryFailure(error);
			return;
		} finally {
			this.forwarding = false;
		}

		try {
			await delivery;
		} catch (error) {
			this.reportDeliveryFailure(error);
		}
	}

	/**
	 * Log de nivel DEBUG
	 */
	debug(message: string, data?: any) {
		this.sendLog(LogLevel.DEBUG, message, data);
	}

	/**
	 * Log de nivel INFO
	 */
	info(message: string, data?: any) {
		this.sendLog(LogLevel.INFO, message, data);
	}

	/**
	 * Log de nivel WARNING
	 */
	warning(message: string, data?: any) {
		this.sendLog(LogLevel.WARNING, message, data);
	}

	/**
	 * Log de nivel ERROR
	 */
	error(message: string, data?: any) {
		this.sendLog(LogLevel.ERROR, message, data);
	}

	/**
	 * Obtiene la ruta del archivo de log actual
	 */
	async getLogFilePath(): Promise<string> {
		try {
			return await getLogFilePath();
		} catch (error) {
			// Por la vía propia y no por `console.error`, que está reemplazada
			// por este mismo logger.
			this.error('Error al obtener la ruta del log', error);
			return '';
		}
	}

	/**
	 * Lee el contenido completo del archivo de log
	 */
	async readLogFile(): Promise<string> {
		try {
			return await readLogFile();
		} catch (error) {
			this.error('Error al leer el archivo de log', error);
			return '';
		}
	}

	/**
	 * Obtiene las últimas N líneas del log
	 */
	async getLastLogLines(lines: number = 100): Promise<string[]> {
		try {
			return await getLastLogLines({ lines });
		} catch (error) {
			this.error('Error al obtener las últimas líneas del log', error);
			return [];
		}
	}
}

// Exportar instancia singleton del logger
export const logger = new VasakLogger();

// Exportar funciones de conveniencia
export const logDebug = (message: string, data?: any) => logger.debug(message, data);
export const logInfo = (message: string, data?: any) => logger.info(message, data);
export const logWarning = (message: string, data?: any) => logger.warning(message, data);
export const logError = (message: string, data?: any) => logger.error(message, data);

// Exportar el logger como default
export default logger;
