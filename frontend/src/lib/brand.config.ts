/**
 * White-label brand configuration.
 *
 * Edit this file to rebrand the entire platform.
 * All components read their branding values from here so a single change
 * propagates everywhere (company name, colors, logo, support email, etc.).
 */

export const brand = {
	/** Displayed in the sidebar header, login page, and browser tab. */
	name: 'Camera Manager',

	/** Short tagline shown below the name on the login page. */
	tagline: 'Video Surveillance Platform',

	/** Appears in <title> and meta tags. */
	windowTitle: 'Camera Manager',

	/** Support / contact email shown in forgot-password flow. */
	supportEmail: 'support@example.com',

	/** URL the logo links to (or null to disable). */
	logoUrl: null as string | null,

	/**
	 * Primary accent colour used for buttons, active states, links, etc.
	 * Must be a valid CSS colour value.  The layout.css @theme block
	 * references this at build time but you can also override at runtime
	 * via CSS custom-properties (see below).
	 */
	colors: {
		primary: '#3b82f6',
		primaryHover: '#2563eb',
		primaryForeground: '#ffffff',
	},

	/**
	 * Override the full light / dark palettes here.
	 * Any key left `undefined` falls back to the defaults in layout.css.
	 *
	 * The keys map 1-to-1 to the --th-* custom properties.
	 */
	lightOverrides: {} as Record<string, string>,
	darkOverrides: {} as Record<string, string>,

	/**
	 * Footer text.  Set to '' to hide the footer.
	 */
	footer: '',

	/**
	 * If true the "Forgot password?" link is shown on the login page.
	 * Requires backend Resend integration to actually send emails.
	 */
	enablePasswordReset: true,

	/**
	 * Resend "from" address used in password-reset emails.
	 * Only relevant on the backend side but kept here for documentation.
	 */
	emailFrom: 'noreply@example.com',
} as const;

export type Brand = typeof brand;
