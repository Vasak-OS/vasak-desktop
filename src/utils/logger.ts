import { invoke } from '@tauri-apps/api/core';

/**
 * Niveles de log disponibles
 */
export enum LogLevel {
  DEBUG = 'DEBUG',
  INFO = 'INFO',
  WARNING = 'WARNING',
  ERROR = 'ERROR',
}

/**
 * Interfaz para el logger de Vasak Desktop
 */
class VasakLogger {
	private isDevelopment: boolean;

	constructor() {
		this.isDevelopment = import.meta.env.DEV;
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
		const originalError = console.error;
		const originalWarn = console.warn;
		const originalLog = console.log;
		const originalDebug = console.debug;

		console.error = (...args: any[]) => {
			this.error(this.formatArgs(args));
			if (this.isDevelopment) {
				originalError.apply(console, args);
			}
		};

		console.warn = (...args: any[]) => {
			this.warning(this.formatArgs(args));
			if (this.isDevelopment) {
				originalWarn.apply(console, args);
			}
		};

		console.log = (...args: any[]) => {
			this.info(this.formatArgs(args));
			if (this.isDevelopment) {
				originalLog.apply(console, args);
			}
		};

		console.debug = (...args: any[]) => {
			this.debug(this.formatArgs(args));
			if (this.isDevelopment) {
				originalDebug.apply(console, args);
			}
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
   * Envía un log al backend de Rust
   */
	private async sendLog(level: LogLevel, message: string, data?: any) {
		const fullMessage = data
			? `${message} | Data: ${JSON.stringify(data)}`
			: message;

		try {
			await invoke('log_from_frontend', {
				level: level.toString(),
				message: fullMessage,
			});
		} catch (error) {
			// Si falla el envío al backend, al menos registrar en consola (solo en dev)
			if (this.isDevelopment) {
				console.warn('[Logger] No se pudo enviar el log al backend:', error);
			}
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
			return await invoke<string>('get_log_file_path');
		} catch (error) {
			console.error('Error al obtener la ruta del log:', error);
			return '';
		}
	}

	/**
   * Lee el contenido completo del archivo de log
   */
	async readLogFile(): Promise<string> {
		try {
			return await invoke<string>('read_log_file');
		} catch (error) {
			console.error('Error al leer el archivo de log:', error);
			return '';
		}
	}

	/**
   * Obtiene las últimas N líneas del log
   */
	async getLastLogLines(lines: number = 100): Promise<string[]> {
		try {
			return await invoke<string[]>('get_last_log_lines', { lines });
		} catch (error) {
			console.error('Error al obtener las últimas líneas del log:', error);
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
