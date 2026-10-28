export function createPanelTabs() {
  for (const list of document.querySelectorAll('[role="tablist"]')) {
    const tabs = [...list.querySelectorAll('[role="tab"]')];
    if (tabs.length === 0) throw new Error('Panel tabs: empty tablist');

    function activate(selected, focus = false) {
      for (const tab of tabs) {
        const active = tab === selected;
        const panel = document.getElementById(tab.getAttribute('aria-controls'));
        if (!panel) throw new Error(`Panel tabs: missing panel for #${tab.id}`);
        tab.setAttribute('aria-selected', String(active));
        tab.tabIndex = active ? 0 : -1;
        panel.hidden = !active;
      }
      if (focus) selected.focus();
    }

    for (const [index, tab] of tabs.entries()) {
      tab.addEventListener('click', () => activate(tab));
      tab.addEventListener('keydown', (event) => {
        let next;
        if (event.key === 'ArrowRight') next = (index + 1) % tabs.length;
        else if (event.key === 'ArrowLeft') next = (index - 1 + tabs.length) % tabs.length;
        else if (event.key === 'Home') next = 0;
        else if (event.key === 'End') next = tabs.length - 1;
        else return;
        event.preventDefault();
        activate(tabs[next], true);
      });
    }
  }
}
