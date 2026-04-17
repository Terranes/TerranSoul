/**
 * Lightweight markdown-to-HTML renderer for chat bubbles and dialogs.
 *
 * XSS Safety: Content is first escaped via escapeHtml() which replaces
 * all &, <, >, " characters with HTML entities. Only then are markdown
 * patterns converted to safe, known HTML tags (<strong>, <em>, <code>,
 * <pre>). No raw user content is ever inserted as HTML.
 */

export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export function renderMarkdown(text: string): string {
  let html = escapeHtml(text);
  // Code blocks (```...```)
  html = html.replace(/```(\w*)\n?([\s\S]*?)```/g, '<pre class="md-code-block"><code>$2</code></pre>');
  // Inline code (`...`)
  html = html.replace(/`([^`]+)`/g, '<code class="md-inline-code">$1</code>');
  // Bold (**...** or __...__) — must come before italic
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/__(.+?)__/g, '<strong>$1</strong>');
  // Italic (*...* or _..._) — uses simple non-greedy match for broad
  // browser compatibility (avoids lookbehind which Safari <16.4 lacks).
  html = html.replace(/\*([^*]+)\*/g, '<em>$1</em>');
  html = html.replace(/\b_([^_]+)_\b/g, '<em>$1</em>');
  // Line breaks
  html = html.replace(/\n/g, '<br/>');
  return html;
}
