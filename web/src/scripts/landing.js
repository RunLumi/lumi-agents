// Progressive enhancement only. No analytics, storage, remote requests or agents.
function enhanceLanding() {
  const tabs = document.querySelector('[data-demo-tabs]');
  if (tabs instanceof HTMLElement && !tabs.dataset.ready) {
    const buttons = [...tabs.querySelectorAll('[data-demo-tab]')];
    const status = document.querySelector('[data-demo-status]');
    const activate = (button, focus = false) => {
      for (const candidate of buttons) {
        const selected = candidate === button;
        candidate.setAttribute('aria-selected', String(selected));
        candidate.setAttribute('tabindex', selected ? '0' : '-1');
        const panel = document.getElementById(candidate.getAttribute('aria-controls') || '');
        if (panel) panel.hidden = !selected;
      }
      if (status) status.textContent = `Ví dụ minh họa: ${button.textContent.trim()}. Không có tác vụ thật được thực thi.`;
      if (focus) button.focus();
    };
    tabs.addEventListener('click', event => {
      const button = event.target instanceof Element ? event.target.closest('[data-demo-tab]') : null;
      if (button && buttons.includes(button)) activate(button);
    });
    tabs.addEventListener('keydown', event => {
      const current = buttons.indexOf(document.activeElement);
      if (current < 0) return;
      let next;
      if (event.key === 'ArrowRight') next = (current + 1) % buttons.length;
      else if (event.key === 'ArrowLeft') next = (current - 1 + buttons.length) % buttons.length;
      else if (event.key === 'Home') next = 0;
      else if (event.key === 'End') next = buttons.length - 1;
      else return;
      event.preventDefault();
      activate(buttons[next], true);
    });
    tabs.dataset.ready = 'true';
    tabs.hidden = false;
  }
  const menu = document.querySelector('.mobile-nav');
  if (menu instanceof HTMLDetailsElement && !menu.dataset.ready) {
    menu.addEventListener('click', event => {
      if (event.target instanceof Element && event.target.closest('a')) menu.open = false;
    });
    menu.addEventListener('keydown', event => {
      if (event.key === 'Escape' && menu.open) {
        menu.open = false;
        menu.querySelector('summary')?.focus();
      }
    });
    document.addEventListener('click', event => {
      if (event.target instanceof Node && !menu.contains(event.target)) menu.open = false;
    });
    menu.dataset.ready = 'true';
  }
}
if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', enhanceLanding, { once: true });
else enhanceLanding();
