/**
 * Theme transition interruption utility.
 *
 * Cancels in-progress CSS color transitions on `.theme-transition` elements
 * before applying new theme values. Only finishes transitions targeting theme
 * color properties, leaving unrelated transform, opacity, shadow, hover, and
 * other animations intact. This prevents sequential animation queueing when
 * rapid theme switches occur.
 *
 * Call `cancelRunningThemeTransitions()` BEFORE toggling the theme class on
 * the document root.
 */

const transitionColorProperties = [
	'color',
	'background-color',
	'border-color',
	'border-top-color',
	'border-right-color',
	'border-bottom-color',
	'border-left-color',
	'outline-color',
	'fill',
	'stroke',
	'stop-color',
	'caret-color',
	'text-decoration-color',
	'column-rule-color',
];

export function cancelRunningThemeTransitions(): void {
	const elements = document.querySelectorAll('.theme-transition');
	elements.forEach((el) => {
		const animations = el.getAnimations();
		for (const anim of animations) {
			if (
				anim instanceof CSSTransition &&
				transitionColorProperties.includes(anim.transitionProperty)
			) {
				anim.finish();
			}
		}
	});
}
