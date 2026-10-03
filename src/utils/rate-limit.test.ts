import { describe, expect, test } from 'bun:test';
import { rateLimited } from './rate-limit';

/** Un reloj y un planificador de mentira, para avanzar el tiempo a mano. */
function fakeTime() {
	let now = 0;
	const queue: Array<{ at: number; fn: () => void; id: number }> = [];
	let ids = 0;
	return {
		clock: () => now,
		schedule: (fn: () => void, ms: number) => {
			ids += 1;
			queue.push({ at: now + ms, fn, id: ids });
			return ids;
		},
		cancel: (id: unknown) => {
			const index = queue.findIndex((entry) => entry.id === id);
			if (index !== -1) queue.splice(index, 1);
		},
		advance(ms: number) {
			now += ms;
			for (const entry of [...queue].sort((a, b) => a.at - b.at)) {
				if (entry.at <= now) {
					queue.splice(queue.indexOf(entry), 1);
					entry.fn();
				}
			}
		},
	};
}

describe('el límite de llamadas por segundo', () => {
	test('la primera sale en el acto', () => {
		const time = fakeTime();
		const sent: Array<[number, number]> = [];
		const limiter = rateLimited<number, number>(
			(k, v) => sent.push([k, v]),
			33,
			time.clock,
			time.schedule,
			time.cancel
		);
		limiter.push(0, 1);
		expect(sent).toEqual([[0, 1]]);
	});

	test('arrastrando, sale como mucho una cada intervalo y siempre la última', () => {
		const time = fakeTime();
		const sent: Array<[number, number]> = [];
		const limiter = rateLimited<number, number>(
			(k, v) => sent.push([k, v]),
			33,
			time.clock,
			time.schedule,
			time.cancel
		);
		// Un arrastre de 100 ms con un evento cada 5 ms: 21 valores.
		for (let i = 0; i <= 20; i++) {
			limiter.push(0, i);
			time.advance(5);
		}
		time.advance(100);
		expect(sent.length).toBeLessThanOrEqual(5);
		expect(sent.at(-1)).toEqual([0, 20]);
	});

	test('cada banda lleva su cuenta', () => {
		const time = fakeTime();
		const sent: Array<[number, number]> = [];
		const limiter = rateLimited<number, number>(
			(k, v) => sent.push([k, v]),
			33,
			time.clock,
			time.schedule,
			time.cancel
		);
		limiter.push(0, 1);
		limiter.push(1, 2);
		expect(sent).toEqual([
			[0, 1],
			[1, 2],
		]);
	});

	test('flush manda lo pendiente sin esperar', () => {
		const time = fakeTime();
		const sent: Array<[number, number]> = [];
		const limiter = rateLimited<number, number>(
			(k, v) => sent.push([k, v]),
			33,
			time.clock,
			time.schedule,
			time.cancel
		);
		limiter.push(0, 1);
		limiter.push(0, 2);
		limiter.flush();
		expect(sent).toEqual([
			[0, 1],
			[0, 2],
		]);
		time.advance(100);
		expect(sent).toHaveLength(2);
	});
});
