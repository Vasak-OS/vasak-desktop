/**
 * Theme transition interruption utility.
 *
 * Cancels in-progress CSS color transitions on `.theme-transition` elements
 * before applying new theme values. This prevents sequential animation queueing
 * when rapid theme switches occur.
 *
 * Call `cancelRunningThemeTransitions()` BEFORE toggling the theme class on
 * the document root.
 */
export function cancelRunningThemeTransitions(): void {
	const elements = document.querySelectorAll('.theme-transition');
	elements.forEach((el) => {
		const animations = el.getAnimations();
		for (const anim of animations) {
			if (anim instanceof CSSTransition) {
				anim.finish();
			}
		}
	});
}
