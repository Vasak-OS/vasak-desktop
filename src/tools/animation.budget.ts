/**
 * Animation Budget Manager
 *
 * Limits simultaneous CSS animations to a maximum of 10 per window.
 * Manages will-change lifecycle and provides queuing with 500ms discard timeout.
 *
 * @requirements 15.1, 15.2, 15.4
 */

class AnimationBudgetManager {
	private activeCount = 0;
	private readonly MAX_ACTIVE = 10;
	private readonly queue: Array<{
		element: HTMLElement;
		resolve: (allowed: boolean) => void;
		timer: ReturnType<typeof setTimeout>;
	}> = [];

	/**
	 * Request an animation slot. Resolves `true` when a slot is available.
	 * If the slot is not available within 500ms, resolves `false` and the
	 * queued animation is discarded.
	 */
	requestSlot(element: HTMLElement): Promise<boolean> {
		if (this.activeCount < this.MAX_ACTIVE) {
			this.activeCount++;
			return Promise.resolve(true);
		}

		return new Promise<boolean>((resolve) => {
			const timer = setTimeout(() => {
				const idx = this.queue.findIndex((q) => q.element === element);
				if (idx !== -1) {
					this.queue.splice(idx, 1);
				}
				resolve(false);
			}, 500);

			this.queue.push({ element, resolve, timer });
		});
	}

	/**
	 * Release a slot when an animation completes and start next queued animation.
	 */
	releaseSlot(): void {
		if (this.activeCount > 0) {
			this.activeCount--;
		}

		if (this.queue.length > 0 && this.activeCount < this.MAX_ACTIVE) {
			const next = this.queue.shift();
			if (next) {
				clearTimeout(next.timer);
				this.activeCount++;
				next.resolve(true);
			}
		}
	}

	/**
	 * Manage will-change property on elements with running animations/transitions.
	 * Adds will-change on start, removes within 100ms of completion.
	 * Uses a WeakMap to track pending reset timeouts per element, preventing stale
	 * callbacks from resetting will-change after an animation restart.
	 */
	private resetTimers = new WeakMap<HTMLElement, ReturnType<typeof setTimeout>>();

	manageWillChange(element: HTMLElement, running: boolean): void {
		const existing = this.resetTimers.get(element);
		if (existing !== undefined) {
			clearTimeout(existing);
			this.resetTimers.delete(element);
		}

		if (running) {
			element.style.willChange = 'transform, opacity';
		} else {
			const timer = setTimeout(() => {
				element.style.willChange = 'auto';
				this.resetTimers.delete(element);
			}, 100);
			this.resetTimers.set(element, timer);
		}
	}

	/** Current count of active animation slots in use */
	get active(): number {
		return this.activeCount;
	}

	/** Current count of queued animations waiting for a slot */
	get queued(): number {
		return this.queue.length;
	}
}

export const animationBudget = new AnimationBudgetManager();
