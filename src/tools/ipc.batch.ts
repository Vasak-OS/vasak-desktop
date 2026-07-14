import { invoke } from '@tauri-apps/api/core';

interface BatchedInvoke {
	command: string;
	args: Record<string, unknown>;
	resolve: (value: any) => void;
	reject: (reason: any) => void;
}

interface BatchRequest {
	id: number;
	command: string;
	args: Record<string, unknown>;
}

interface BatchResponse {
	id: number;
	success: boolean;
	data?: unknown;
	error?: string;
}

class IPCBatchLayer {
	private readonly queue: BatchedInvoke[] = [];
	private scheduled = false;
	private readonly MAX_BATCH_SIZE = 20;
	private readonly TIMEOUT_MS = 500;

	/**
	 * Queues an invoke call and returns a Promise that resolves with the result.
	 * Dispatches are batched within a single microtask/frame boundary (16ms).
	 */
	invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
		return new Promise<T>((resolve, reject) => {
			this.queue.push({
				command,
				args: args ?? {},
				resolve,
				reject,
			});
			this.scheduleFlush();
		});
	}

	/**
	 * Schedules a flush on the next microtask boundary.
	 * Uses queueMicrotask to collect all invocations made within the same frame.
	 */
	private scheduleFlush(): void {
		if (this.scheduled) return;
		this.scheduled = true;
		queueMicrotask(() => {
			this.flush();
		});
	}

	/**
	 * Dispatches up to MAX_BATCH_SIZE requests per batch via a single `batch_invoke` Tauri command.
	 * Implements 500ms timeout per batch and per-command failure isolation.
	 */
	private async flush(): Promise<void> {
		this.scheduled = false;

		while (this.queue.length > 0) {
			const batch = this.queue.splice(0, this.MAX_BATCH_SIZE);
			await this.dispatchBatch(batch);
		}
	}

	/**
	 * Dispatches a single batch of requests with timeout and per-command error isolation.
	 */
	private async dispatchBatch(batch: BatchedInvoke[]): Promise<void> {
		const requests: BatchRequest[] = batch.map((item, index) => ({
			id: index,
			command: item.command,
			args: item.args,
		}));

		let timeoutId: ReturnType<typeof setTimeout> | null = null;

		const timeoutPromise = new Promise<never>((_resolve, reject) => {
			timeoutId = setTimeout(() => {
				reject(new Error(`IPC batch timed out after ${this.TIMEOUT_MS}ms`));
			}, this.TIMEOUT_MS);
		});

		try {
			const responses = (await Promise.race([
				invoke<BatchResponse[]>('batch_invoke', { requests }),
				timeoutPromise,
			])) as BatchResponse[];

			if (timeoutId !== null) clearTimeout(timeoutId);

			// Per-command failure isolation: resolve/reject each promise individually
			for (const response of responses) {
				const entry = batch[response.id];
				if (!entry) continue;

				if (response.success) {
					entry.resolve(response.data);
				} else {
					entry.reject(new Error(response.error ?? `Command "${entry.command}" failed`));
				}
			}

			// Handle any requests that didn't get a response
			for (let i = 0; i < batch.length; i++) {
				const hasResponse = responses.some((r) => r.id === i);
				if (!hasResponse) {
					batch[i].reject(new Error(`No response received for command "${batch[i].command}"`));
				}
			}
		} catch (error) {
			if (timeoutId !== null) clearTimeout(timeoutId);

			// On timeout or transport error, reject all promises in the batch
			for (const entry of batch) {
				entry.reject(error);
			}
		}
	}
}

export const ipcBatch = new IPCBatchLayer();
