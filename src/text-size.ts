// This limits visual expansion, never the shared content or selection.
export function fitTextArea(text: HTMLTextAreaElement) {
  const style = getComputedStyle(text);
  const probe = document.createElement('textarea');
  probe.tabIndex = -1;
  probe.setAttribute('aria-hidden', 'true');
  probe.wrap = text.wrap;
  Object.assign(probe.style, {
    position: 'fixed', visibility: 'hidden', pointerEvents: 'none',
    top: '0', left: '0', width: `${text.getBoundingClientRect().width}px`,
    height: '0', minHeight: '0', maxHeight: 'none', overflow: 'hidden',
    boxSizing: style.boxSizing, font: style.font, lineHeight: style.lineHeight,
    letterSpacing: style.letterSpacing, padding: style.padding, border: style.border,
  });
  probe.value = Array.from(text.value).slice(0, 1000).join('') + ' ';
  document.body.append(probe);
  const desired = probe.scrollHeight + 2;
  probe.remove();
  const maximum = Math.max(180, Math.min(600, Math.floor(window.innerHeight * .65)));
  const height = `${Math.max(180, Math.min(desired, maximum))}px`;
  // Avoid ResizeObserver loops and preserve the textarea's focus/selection.
  if (text.style.height !== height) text.style.height = height;
}
