const DAYS = ['일', '월', '화', '수', '목', '금', '토'];

export default {
  kind: 'calendar',
  label: '달력',
  note: '이번 달 달력을 보여주고 오늘 날짜를 표시해요.',
  defaults: { weekStart: '0' },
  fields: [{ key: 'weekStart', label: '한 주의 시작', type: 'select', options: [['0', '일요일'], ['1', '월요일']] }],
  css: `
    .k-calendar { display: flex; flex-direction: column; padding: 6cqmin 7cqmin; gap: 3cqmin; }
    .k-calendar .title { font-size: 8cqmin; font-weight: 600; }
    .k-calendar .title span { color: var(--muted); font-weight: 400; margin-left: 0.4em; }
    .k-calendar .grid { flex: 1; display: grid; grid-template-columns: repeat(7, 1fr); grid-auto-rows: 1fr; font-size: 5.4cqmin; font-variant-numeric: tabular-nums; }
    .k-calendar .grid > * { display: flex; align-items: center; justify-content: center; }
    .k-calendar .dow { color: var(--muted); font-size: 4.4cqmin; }
    .k-calendar .sun { color: var(--danger); }
    .k-calendar .today span { background: var(--accent); color: var(--on-accent); font-weight: 700; border-radius: 50%; width: 1.9em; height: 1.9em; display: flex; align-items: center; justify-content: center; }
  `,
  mount(root, ctx) {
    const start = Number(ctx.options.weekStart);
    let shown = '';

    function draw() {
      const now = new Date();
      const key = now.toDateString();
      if (key === shown) return;
      shown = key;
      const year = now.getFullYear();
      const month = now.getMonth();
      const lead = (new Date(year, month, 1).getDay() - start + 7) % 7;
      const count = new Date(year, month + 1, 0).getDate();

      const title = document.createElement('div');
      title.className = 'title';
      title.innerHTML = `${month + 1}월<span>${year}</span>`;
      const grid = document.createElement('div');
      grid.className = 'grid';
      for (let i = 0; i < 7; i++) {
        const day = (start + i) % 7;
        const cell = document.createElement('div');
        cell.className = day === 0 ? 'dow sun' : 'dow';
        cell.textContent = DAYS[day];
        grid.append(cell);
      }
      for (let i = 0; i < lead; i++) grid.append(document.createElement('div'));
      for (let date = 1; date <= count; date++) {
        const cell = document.createElement('div');
        const day = (lead + date - 1 + start) % 7;
        cell.className = [day === 0 ? 'sun' : '', date === now.getDate() ? 'today' : ''].join(' ').trim();
        const text = document.createElement('span');
        text.textContent = date;
        cell.append(text);
        grid.append(cell);
      }
      root.replaceChildren(title, grid);
    }

    draw();
    const timer = setInterval(draw, 30000);
    return () => clearInterval(timer);
  },
};
