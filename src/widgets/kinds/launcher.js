import { invoke } from '@tauri-apps/api/core';

export default {
  kind: 'launcher',
  label: '바로가기',
  note: '자주 쓰는 프로그램, 폴더, 웹 주소를 눌러서 열어요.',
  defaults: { items: [] },
  fields: [
    {
      key: 'items',
      label: '바로가기',
      type: 'list',
      addLabel: '바로가기 추가',
      item: [
        { key: 'name', label: '이름', type: 'text', placeholder: '이름' },
        { key: 'target', label: '대상', type: 'file', placeholder: '프로그램, 폴더 또는 웹 주소', hint: '프로그램·파일·폴더 경로나 웹 주소' },
      ],
    },
  ],
  css: `
    .k-launcher { padding: 4cqmin; overflow: hidden; }
    .k-launcher .grid { height: 100%; display: grid; grid-template-columns: repeat(auto-fill, 28cqmin); grid-auto-rows: 28cqmin; justify-content: center; align-content: center; gap: 2.5cqmin; overflow: hidden; }
    .k-launcher .tile { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1.5cqmin; min-width: 0; min-height: 0; background: var(--raised); border: 1px solid var(--line); border-radius: 2.5cqmin; color: var(--text); font: inherit; cursor: pointer; padding: 1cqmin; }
    .k-launcher .tile:hover { border-color: var(--accent); }
    .k-launcher .tile:active { background: var(--line); }
    .k-launcher .letter { font-size: 9cqmin; font-weight: 200; line-height: 1; color: var(--accent); }
    .k-launcher .name { font-size: 4.2cqmin; max-width: 100%; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .k-launcher .tile.fail { border-color: var(--danger); }
    .k-launcher .empty { height: 100%; display: flex; align-items: center; justify-content: center; text-align: center; color: var(--muted); font-size: 5.6cqmin; line-height: 1.5; }
  `,
  mount(root, ctx) {
    const items = (ctx.options.items ?? []).filter((it) => it?.target?.trim());
    if (!items.length) {
      const empty = document.createElement('div');
      empty.className = 'empty';
      empty.textContent = '설정에서 바로가기를 추가해 보세요.';
      root.replaceChildren(empty);
      return () => {};
    }
    const grid = document.createElement('div');
    grid.className = 'grid';
    const timers = [];
    let alive = true;
    for (const it of items) {
      const label = (it.name || it.target).trim();
      const tile = document.createElement('button');
      tile.className = 'tile';
      tile.title = it.target;
      const letter = document.createElement('span');
      letter.className = 'letter';
      letter.textContent = [...label][0].toUpperCase();
      const name = document.createElement('span');
      name.className = 'name';
      name.textContent = label;
      tile.append(letter, name);
      tile.addEventListener('click', async () => {
        try {
          await invoke('open_target', { target: it.target });
        } catch {
          if (!alive) return;
          tile.classList.add('fail');
          name.textContent = '열 수 없어요';
          timers.push(
            setTimeout(() => {
              tile.classList.remove('fail');
              name.textContent = label;
            }, 2500),
          );
        }
      });
      grid.append(tile);
    }
    root.replaceChildren(grid);
    return () => {
      alive = false;
      timers.forEach(clearTimeout);
    };
  },
};
