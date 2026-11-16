const DAYS = ['일', '월', '화', '수', '목', '금', '토'];

export default {
  kind: 'dday',
  label: '디데이',
  note: '정해 둔 날까지 남은 날수를 크게 보여줘요.',
  defaults: { title: '', date: '' },
  fields: [
    { key: 'title', label: '제목', type: 'text', placeholder: '예: 여행 출발' },
    { key: 'date', label: '날짜', type: 'date' },
  ],
  css: `
    .k-dday { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3cqmin; padding: 6cqmin; text-align: center; }
    .k-dday .title { font-size: 8cqmin; font-weight: 600; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .k-dday .count { font-size: 30cqmin; font-weight: 200; line-height: 1; font-variant-numeric: tabular-nums; white-space: nowrap; }
    .k-dday .count.today { color: var(--accent); }
    .k-dday .date { font-size: 5cqmin; color: var(--muted); }
    .k-dday .empty { font-size: 6cqmin; color: var(--muted); line-height: 1.5; }
  `,
  mount(root, ctx) {
    const { title, date } = ctx.options;
    const target = /^\d{4}-\d{2}-\d{2}$/.test(date) ? date.split('-').map(Number) : null;
    let shown = '';

    function draw() {
      const now = new Date();
      const key = now.toDateString();
      if (key === shown) return;
      shown = key;
      if (!target) {
        const empty = document.createElement('div');
        empty.className = 'empty';
        empty.textContent = '설정에서 날짜를 정해 주세요.';
        root.replaceChildren(empty);
        return;
      }
      const [y, m, d] = target;
      const days = Math.round((Date.UTC(y, m - 1, d) - Date.UTC(now.getFullYear(), now.getMonth(), now.getDate())) / 86400000);
      const name = document.createElement('div');
      name.className = 'title';
      name.textContent = title || '디데이';
      const count = document.createElement('div');
      count.className = days === 0 ? 'count today' : 'count';
      count.textContent = days === 0 ? 'D-DAY' : days > 0 ? `D-${days}` : `D+${-days}`;
      const when = document.createElement('div');
      when.className = 'date';
      when.textContent = `${y}년 ${m}월 ${d}일 (${DAYS[new Date(y, m - 1, d).getDay()]})`;
      root.replaceChildren(name, count, when);
    }

    draw();
    const timer = setInterval(() => {
      if (!ctx.isPaused()) draw();
    }, 30000);
    return () => clearInterval(timer);
  },
};
