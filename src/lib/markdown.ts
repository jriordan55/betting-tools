import { marked } from 'marked';
import katex from 'katex';

/**
 * Markdown with LaTeX, rendered for the in-app reference library.
 *
 * The awkward part is that `$` means three different things in these
 * documents: a math delimiter, a literal dollar sign inside math
 * (`$\$2100$`), and a literal dollar sign in prose. Handing the raw text to a
 * markdown parser first mangles the LaTeX — `_` becomes emphasis, `\\` becomes
 * a line break — so the math is lifted out before parsing and put back after.
 *
 * Inline code is protected the same way and for the same reason: a `$` inside
 * backticks is a dollar sign, not the start of an equation.
 */

interface Extracted {
	/** The text with every protected span replaced by a placeholder. */
	text: string;
	/** Rendered HTML for each placeholder, in order. */
	spans: string[];
}

/**
 * A placeholder no markdown parser will touch and no document will contain.
 *
 * Letters and digits only: anything with punctuation risks being turned into
 * emphasis, a link, or an entity on the way through.
 */
function placeholder(index: number): string {
	return `xKaTeXPlaceholderx${index}x`;
}

function renderMath(source: string, display: boolean): string {
	try {
		return katex.renderToString(source, {
			displayMode: display,
			throwOnError: false,
			// `\$` is how these documents write a literal dollar inside math.
			strict: false
		});
	} catch {
		// A malformed equation must not blank the whole page. Show the source.
		const escaped = source.replace(/&/g, '&amp;').replace(/</g, '&lt;');
		return `<code class="math-error">${escaped}</code>`;
	}
}

/**
 * Lifts math and inline code out of the text, leaving placeholders.
 *
 * Hand-scanned rather than done with a regex because the escapes matter: `\$`
 * is a literal dollar and must not open or close anything.
 */
function extract(source: string): Extracted {
	const spans: string[] = [];
	let out = '';
	let i = 0;

	while (i < source.length) {
		const char = source[i];

		// A backslash escape consumes the next character whatever it is.
		if (char === '\\' && i + 1 < source.length) {
			out += source.slice(i, i + 2);
			i += 2;
			continue;
		}

		// Inline code: a `$` in here is a dollar sign.
		if (char === '`') {
			const fence = /^`+/.exec(source.slice(i))?.[0] ?? '`';
			const close = source.indexOf(fence, i + fence.length);
			if (close === -1) {
				out += source.slice(i);
				break;
			}
			out += source.slice(i, close + fence.length);
			i = close + fence.length;
			continue;
		}

		if (source.startsWith('$$', i)) {
			// Display math is written on its own lines, so the delimiters are
			// allowed to be followed and preceded by whitespace.
			const end = findDisplayClose(source, i + 2);
			if (end !== -1 && source.slice(i + 2, end).trim() !== '') {
				out += placeholder(spans.length);
				spans.push(renderMath(source.slice(i + 2, end).trim(), true));
				i = end + 2;
				continue;
			}
		} else if (char === '$') {
			const end = findInlineClose(source, i + 1);
			if (end !== -1) {
				out += placeholder(spans.length);
				spans.push(renderMath(source.slice(i + 1, end).trim(), false));
				i = end + 1;
				continue;
			}
		}

		out += char;
		i += 1;
	}

	return { text: out, spans };
}

/** Index of the closing `$$`, or -1. */
function findDisplayClose(source: string, from: number): number {
	for (let i = from; i < source.length; i += 1) {
		if (source[i] === '\\') {
			i += 1;
			continue;
		}
		if (source.startsWith('$$', i)) return i;
	}
	return -1;
}

/**
 * Index of the closing `$` of an inline equation, or -1.
 *
 * The rule is the usual one, and it exists entirely to keep currency out of
 * the maths: an opening `$` must be followed by a non-space and a closing `$`
 * must be *preceded* by one. That is what stops `a $100 bet returns $190.91`
 * from parsing as an equation whose body is " bet returns " — which is not an
 * error anyone would see, just a sentence quietly rendered as algebra.
 *
 * A closing `$` may not be followed by a digit either, so `$5 and $10` stays
 * two prices rather than becoming one equation.
 */
function findInlineClose(source: string, from: number): number {
	// `$ x$` is prose. An equation starts immediately after the delimiter.
	if (from >= source.length || /\s/.test(source[from])) return -1;

	for (let i = from; i < source.length; i += 1) {
		if (source[i] === '\\') {
			i += 1;
			continue;
		}
		// An inline equation never spans a blank line; a `$` left open in prose
		// would otherwise consume the rest of the document.
		if (source.startsWith('\n\n', i)) return -1;
		if (source[i] !== '$') continue;

		const before = source[i - 1];
		const after = source[i + 1];
		if (before !== undefined && !/\s/.test(before) && !/[0-9]/.test(after ?? '')) {
			return i;
		}
	}
	return -1;
}

/** Renders a markdown document with LaTeX to HTML. */
export function renderMarkdown(source: string): string {
	const { text, spans } = extract(source);
	const html = marked.parse(text, { async: false, gfm: true });

	// Placeholders can land inside a paragraph the parser wrapped, so this is
	// a plain string replacement rather than anything structural.
	return spans.reduce(
		(acc, span, index) => acc.replaceAll(placeholder(index), span),
		html as string
	);
}
