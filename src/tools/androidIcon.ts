/**
 * Icons for the phone's applications.
 *
 * Android does not hand out an app's icon through any shell command, so the
 * device service cannot send one and `ConnectApp.icon` arrives empty. Every
 * entry therefore fell back to the same generic executable icon, and a list of
 * ninety identical squares is worse than no icons at all — it looks broken and
 * tells you nothing.
 *
 * Two things fix that without asking the phone anything.
 *
 * **A map, on purpose curated and small.** Deriving a theme icon name from the
 * package was measured on a real phone and it is a trap: it matches about a
 * third of the apps, and most of those matches are wrong. `com.android.settings`
 * lands on `settings`, which is *VasakOS'* settings icon — right next to a row
 * labelled "Ajustes", so it reads as if it opened the local one. Same for
 * calculator, calendar, contacts and photos. Only entries whose brand icon is
 * unmistakably the same product are listed here.
 *
 * **An initial for everything else.** A tile with the app's first letter, in the
 * session's own colours. It distinguishes one row from the next, which is the
 * whole job, and it never claims to be something it is not.
 */

/**
 * Package name → icon name in the icon theme.
 *
 * Every value was checked against the installed VasakOS theme; a name that is
 * not there would silently fall through to the initial, which is a confusing way
 * to discover a typo.
 */
const THEME_ICONS: Record<string, string> = {
	// Messaging
	'com.whatsapp': 'whatsapp',
	'com.whatsapp.w4b': 'whatsapp',
	'org.telegram.messenger': 'telegram',
	'org.telegram.messenger.web': 'telegram',
	'org.thoughtcrime.securesms': 'signal-desktop',
	'com.viber.voip': 'viber',
	'com.tencent.mm': 'wechat',
	'com.discord': 'discord',
	'com.Slack': 'slack',
	'com.microsoft.teams': 'teams',
	'com.skype.raider': 'skype',
	'us.zoom.videomeetings': 'zoom',

	// Social
	'com.instagram.android': 'instagram',
	'com.twitter.android': 'twitter',
	'com.reddit.frontpage': 'reddit',
	'com.linkedin.android': 'linkedin',

	// Media
	'com.spotify.music': 'spotify',
	'com.google.android.youtube': 'youtube',
	'com.netflix.mediaclient': 'netflix',
	'org.videolan.vlc': 'vlc',

	// Google, only where the brand icon is the same product
	'com.google.android.gm': 'gmail',
	'com.android.chrome': 'google-chrome',
	'com.google.android.apps.docs': 'google-drive',

	// Browsers
	'org.mozilla.firefox': 'firefox',
	'org.mozilla.fenix': 'firefox',

	// Storage
	'com.dropbox.android': 'dropbox',
};

/**
 * The theme icon for a package, or `null` when there is none.
 *
 * Returning `null` rather than a generic name is deliberate: the caller shows an
 * initial instead, and a wrong icon is worse than no icon.
 */
export function themeIconFor(pkg: string): string | null {
	return THEME_ICONS[pkg] ?? null;
}

/**
 * The letter shown when there is no icon.
 *
 * Taken from the label rather than the package, because that is the word the
 * person is reading on the same row. Falls back to the package for an app whose
 * name is only symbols or emoji, where the first character carries nothing.
 */
export function initialFor(label: string, pkg: string): string {
	const source = /\p{Letter}|\p{Number}/u.test(label) ? label : pkg;
	const match = source.match(/\p{Letter}|\p{Number}/u);
	return (match?.[0] ?? '?').toLocaleUpperCase();
}
