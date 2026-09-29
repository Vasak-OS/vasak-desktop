import { describe, expect, test } from 'bun:test';
import { createSerialQueue } from './serial-queue';

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

describe('la fila de pedidos', () => {
	test('el pedido lento no termina después del rápido que llegó detrás', async () => {
		// Es el caso de la bandeja: el primer icono tarda en contestar su menú y
		// el segundo no. Sin la fila, el último en escribir es el primero.
		const run = createSerialQueue();
		const written: string[] = [];

		await Promise.all([
			run(async () => {
				await wait(30);
				written.push('primero');
			}),
			run(async () => {
				written.push('segundo');
			}),
		]);

		expect(written).toEqual(['primero', 'segundo']);
	});

	test('uno que falla no traba a los que vienen detrás', async () => {
		const run = createSerialQueue();

		const failed = run(async () => {
			throw new Error('sin menú');
		});
		const after = run(async () => 'abierto');

		await expect(failed).rejects.toThrow('sin menú');
		expect(await after).toBe('abierto');
	});
});
