/**
 * View Transition Performance Guard
 *
 * Restricts View Transition API usage to user-initiated theme switches only.
 * Defers non-visual data updates while a transition is in progress.
 * Handles concurrent transitions by cancelling the first and applying only the latest.
 */

interface ViewTransition {
	finished: Promise<void>;
	ready: Promise<void>;
	updateCallbackDone: Promise<void>;
	skipTransition(): void;
}

class ViewTransitionGuard {
	private transitioning = false;
	private readonly deferredCallbacks: Array<() => void | Promise<void>> = [];
	private transitionTimeout: ReturnType<typeof setTimeout> | null = null;
	private currentTransition: ViewTransition | null = null;

	/**
	 * Wraps document.startViewTransition() with guards.
	 * Only call for user-initiated theme switches.
	 *
	 * - Checks browser support before calling
	 * - Sets transitioning state during the transition
	 * - Enforces 500ms force-complete timeout
	 * - On concurrent requests: cancels first, applies only latest
	 */
	async startTransition(updateFn: () => void | Promise<void>): Promise<void> {
		if (typeof document.startViewTransition !== 'function') {
			await updateFn();
			this.flushDeferred();
			return;
		}

		// Cancel any in-progress transition before starting a new one
		if (this.currentTransition) {
			const oldTransition = this.currentTransition;
			oldTransition.skipTransition();
			this.clearTimeout();
			// Wait for the old update callback to settle before proceeding
			await oldTransition.updateCallbackDone.catch(() => {});
			// Don't flush deferred here — the new transition takes over
		}

		this.transitioning = true;

		// Set 500ms force-complete timeout
		this.transitionTimeout = setTimeout(() => {
			this.forceComplete();
		}, 500);

		const transition = document.startViewTransition(updateFn) as unknown as ViewTransition;
		this.currentTransition = transition;

		try {
			await transition.finished;
		} catch (error) {
			if (error instanceof DOMException && error.name === 'AbortError') {
				// Transition was skipped or cancelled via skipTransition() — expected.
			} else {
				console.error('[ViewTransitionGuard] Transition update failed:', error);
			}
		} finally {
			// Only complete if this is still the active transition
			// (a concurrent request may have replaced us)
			if (this.currentTransition === transition) {
				this.complete();
			}
		}
	}

	/**
	 * Defer a non-visual data update while a transition is in progress.
	 * If no transition is active, executes the callback immediately.
	 */
	deferUpdate(callback: () => void | Promise<void>): void {
		if (this.transitioning) {
			this.deferredCallbacks.push(callback);
		} else {
			const result = callback();
			if (result) result.catch((error) => {
				console.error('[ViewTransitionGuard] Deferred callback threw an error:', error);
			});
		}
	}

	/**
	 * Returns whether a View Transition is currently active.
	 */
	isTransitioning(): boolean {
		return this.transitioning;
	}

	/**
	 * Normal completion: transition finished naturally.
	 * Clears timeout and flushes all deferred callbacks.
	 */
	private complete(): void {
		this.clearTimeout();
		this.transitioning = false;
		this.currentTransition = null;
		this.flushDeferred();
	}

	/**
	 * Force completion after 500ms timeout.
	 * Cancels animation and flushes deferred callbacks only after the update
	 * callback has settled.
	 */
	private async forceComplete(): Promise<void> {
		const transition = this.currentTransition;
		if (transition) {
			transition.skipTransition();
			await transition.updateCallbackDone.catch(() => {});
		}
		this.transitionTimeout = null;
		this.transitioning = false;
		this.currentTransition = null;
		this.flushDeferred();
	}

	/**
	 * Flushes all deferred callbacks in order, clearing the queue.
	 */
	private flushDeferred(): void {
		const callbacks = this.deferredCallbacks.splice(0);
		for (const cb of callbacks) {
			try {
				const result = cb();
				if (result) result.catch((error) => {
					console.error('[ViewTransitionGuard] Deferred callback threw an error:', error);
				});
			} catch (error) {
				console.error('[ViewTransitionGuard] Deferred callback threw an error:', error);
			}
		}
	}

	/**
	 * Clears the force-complete timeout if set.
	 */
	private clearTimeout(): void {
		if (this.transitionTimeout !== null) {
			clearTimeout(this.transitionTimeout);
			this.transitionTimeout = null;
		}
	}
}

/** Singleton View Transition guard instance */
export const viewTransitionGuard = new ViewTransitionGuard();
