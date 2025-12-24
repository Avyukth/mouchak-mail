/**
 * Slugify utility for generating URL-friendly identifiers.
 *
 * @module utils/slugify
 */

/**
 * Convert a string to a URL-friendly slug.
 *
 * @param text - The text to convert
 * @returns A lowercase, hyphen-separated slug
 *
 * @example
 * slugify("My Project Name") // "my-project-name"
 * slugify("Hello World!") // "hello-world"
 * slugify("test__multiple---separators") // "test-multiple-separators"
 */
export function slugify(text: string): string {
	return text
		.toLowerCase()
		.trim()
		.replace(/[^\w\s-]/g, '') // Remove special chars except spaces/hyphens
		.replace(/[\s_-]+/g, '-') // Replace spaces/underscores/hyphens with single hyphen
		.replace(/^-+|-+$/g, ''); // Remove leading/trailing hyphens
}

/**
 * Check if a string is a valid slug.
 *
 * @param text - The text to validate
 * @returns True if the text is a valid slug
 */
export function isValidSlug(text: string): boolean {
	return /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(text);
}
