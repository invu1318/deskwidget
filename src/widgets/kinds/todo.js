export default {
  kind: 'todo',
  label: '할 일',
  note: '할 일을 적고 체크하며 관리해요.',
  defaults: { hideDone: false },
  fields: [{ key: 'hideDone', label: '완료한 항목 숨기기', type: 'toggle' }],
  css: `
    .k-todo { display: flex; flex-direction: column; padding: 5cqmin 6cqmin; gap: 3cqmin; }
    .k-todo .head { display: flex; align-items: baseline; justify-content: space-between; font-size: 7cqmin; font-weight: 600; }
    .k-todo .head span { font-size: 5cqmin; font-weight: 400; color: var(--muted); }
    .k-todo .head span b { color: var(--accent); font-weight: 600; }
    .k-todo input { box-sizing: border-box; width: 100%; font: inherit; font-size: 5.5cqmin; padding: 1.6cqmin 2.4cqmin; color: var(--text); background: var(--raised); border: 1px solid var(--line); border-radius: 1.5cqmin; outline: none; }
    .k-todo input:focus { border-color: var(--accent); }
    .k-todo ul { flex: 1; min-height: 0; margin: 0; padding: 0; list-style: none; overflow-y: auto; }
    .k-todo li { display: flex; align-items: center; gap: 2.5cqmin; padding: 1.4cqmin 0; font-size: 5.5cqmin; border-bottom: 1px solid var(--line); }
    .k-todo .box { flex: none; width: 1.1em; height: 1.1em; box-sizing: border-box; border: 1.5px solid var(--muted); border-radius: 50%; background: none; padding: 0; cursor: pointer; color: var(--on-accent); font-size: 0.8em; line-height: 1; }
    .k-todo .done .box { background: var(--accent); border-color: var(--accent); }
    .k-todo .text { flex: 1; min-width: 0; overflow-wrap: anywhere; cursor: pointer; }
    .k-todo .done .text { text-decoration: line-through; color: var(--muted); }
    .k-todo .del { flex: none; background: none; border: 0; color: var(--muted); font-size: 1.1em; line-height: 1; cursor: pointer; padding: 0 0.3em; }
    .k-todo .del:hover { color: var(--danger); }
    .k-todo .none { color: var(--muted); font-size: 5cqmin; padding-top: 2cqmin; }
  `,
  mount(root, ctx) {
    let items = Array.isArray(ctx.data?.items) ? ctx.data.items : [];
    const save = () => ctx.saveData({ items });

    const head = document.createElement('div');
    head.className = 'head';
    const input = document.createElement('input');
    input.placeholder = '할 일 추가';
    input.maxLength = 200;
    input.setAttribute('aria-label', '할 일 추가');
    const list = document.createElement('ul');
    root.replaceChildren(head, input, list);

    function draw() {
      const left = items.filter((i) => !i.done).length;
      head.innerHTML = `할 일<span>남은 일 <b>${left}</b>개</span>`;
      const rows = ctx.options.hideDone ? items.filter((i) => !i.done) : items;
      list.replaceChildren(
        ...rows.map((item) => {
          const li = document.createElement('li');
          if (item.done) li.className = 'done';
          const box = document.createElement('button');
          box.className = 'box';
          box.textContent = item.done ? '✓' : '';
          box.setAttribute('aria-label', '완료 표시');
          const text = document.createElement('span');
          text.className = 'text';
          text.textContent = item.text;
          const del = document.createElement('button');
          del.className = 'del';
          del.textContent = '×';
          del.setAttribute('aria-label', '삭제');
          const toggle = () => {
            item.done = !item.done;
            save();
            draw();
          };
          box.onclick = toggle;
          text.onclick = toggle;
          del.onclick = () => {
            items = items.filter((i) => i !== item);
            save();
            draw();
          };
          li.append(box, text, del);
          return li;
        }),
      );
      if (!rows.length) {
        const none = document.createElement('li');
        none.className = 'none';
        none.textContent = items.length ? '모두 끝냈어요.' : '할 일이 없어요.';
        list.append(none);
      }
    }

    input.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter' || e.isComposing) return;
      const text = input.value.trim();
      if (!text) return;
      items = [...items, { text, done: false }];
      input.value = '';
      save();
      draw();
    });

    draw();
    return () => {};
  },
};
