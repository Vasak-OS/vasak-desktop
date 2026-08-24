import { invoke } from '@tauri-apps/api/core';

/**
 * El pronóstico guardado del lado de Rust, con su edad: alcanza para mostrar
 * algo viejo mientras se pide lo nuevo, en vez de dejar el widget en blanco.
 */
export interface WeatherSnapshot {
	datos: any;
	edad_segundos: number;
	vencido: boolean;
}

export interface WeatherPlace {
	lat: number;
	lon: number;
}

export const weatherCached = (): Promise<WeatherSnapshot | null> => invoke('weather_cached');

export const weatherPlace = (): Promise<WeatherPlace | null> => invoke('weather_place');

/** «¿Me toca pedirlo?». Sólo una ventana recibe `true` por vuelta. */
export const weatherClaim = (): Promise<boolean> => invoke('weather_claim');

export const weatherStore = (datos: any, lugar?: WeatherPlace | null): Promise<void> =>
	invoke('weather_store', { datos, lugar: lugar ?? null });

/** Devuelve el turno cuando el pedido falló, sin esperar a que caduque. */
export const weatherRelease = (): Promise<void> => invoke('weather_release');
