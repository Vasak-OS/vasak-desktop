/** El estado del ecualizador de sistema, como lo manda `applets/equalizer.rs`. */
export interface EqualizerState {
	/** Si el servicio `org.vasak.Equalizer` está en el bus. */
	service: boolean;
	/** Las frecuencias de las bandas, en Hz. */
	frequencies: number[];
	/** `[mínimo, máximo]` en dB. */
	range: [number, number];
	/** Los perfiles de fábrica, en el orden de la grilla. */
	presets: string[];
	/** El elegido: uno de `presets`, o `custom`. */
	preset: string;
	/** Las ganancias que suenan, en dB. */
	gains: number[];
	enabled: boolean;
	/** Si el filtro está en PipeWire. */
	available: boolean;
	/** Si lo que suena es lo que está en disco. */
	saved: boolean;
}

export function missingEqualizer(): EqualizerState {
	return {
		service: false,
		frequencies: [],
		range: [-12, 12],
		presets: [],
		preset: '',
		gains: [],
		enabled: false,
		available: false,
		saved: true,
	};
}
