const ZONES = [
  ['Asia/Seoul', '서울'],
  ['Asia/Tokyo', '도쿄'],
  ['Asia/Shanghai', '상하이'],
  ['Asia/Hong_Kong', '홍콩'],
  ['Asia/Singapore', '싱가포르'],
  ['Asia/Bangkok', '방콕'],
  ['Asia/Dubai', '두바이'],
  ['Asia/Kolkata', '뭄바이'],
  ['Europe/London', '런던'],
  ['Europe/Paris', '파리'],
  ['Europe/Berlin', '베를린'],
  ['Europe/Moscow', '모스크바'],
  ['Africa/Cairo', '카이로'],
  ['Australia/Sydney', '시드니'],
  ['Pacific/Auckland', '오클랜드'],
  ['Pacific/Honolulu', '호놀룰루'],
  ['America/Anchorage', '앵커리지'],
  ['America/Los_Angeles', '로스앤젤레스'],
  ['America/Denver', '덴버'],
  ['America/Chicago', '시카고'],
  ['America/New_York', '뉴욕'],
  ['America/Sao_Paulo', '상파울루'],
];

const fmtCache = new Map();
function parts(zone, date) {
  let f = fmtCache.get(zone);
  if (!f) {
    f = new Intl.DateTimeFormat('en-US', { timeZone: zone, hour12: false, year: 'numeric', month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
    fmtCache.set(zone, f);
  }
  return Object.fromEntries(f.formatToParts(date).map((p) => [p.type, p.value]));
}
const dayNumber = (p) => Date.UTC(+p.year, +p.month - 1, +p.day) / 86400000;

export default {
  kind: 'worldclock',
  label: '세계 시계',
  note: '여러 도시의 현재 시각을 한눈에 보여줘요.',
  defaults: {
    cities: [
      { name: '서울', zone: 'Asia/Seoul' },
      { name: '뉴욕', zone: 'America/New_York' },
      { name: '런던', zone: 'Europe/London' },
    ],
  },
  fields: [
    {
      key: 'cities',
      label: '도시',
      type: 'list',
      addLabel: '도시 추가',
      item: [
        { key: 'name', label: '도시 이름', type: 'text', placeholder: '도시 이름', default: '' },
        { key: 'zone', label: '시간대', type: 'select', options: ZONES.map(([z, n]) => [z, `${n} (${z})`]), default: 'Asia/Seoul' },
      ],
    },
  ],
  css: `
    .k-worldclock { display: flex; flex-direction: column; justify-content: space-evenly; padding: 4cqmin 7cqmin; }
    .k-worldclock .row { display: flex; align-items: baseline; gap: 3cqmin; min-height: 0; }
    .k-worldclock .row { --size: min(11cqmin, calc(62cqh / var(--n, 3))); line-height: 1.3; }
    .k-worldclock .name { flex: 1; min-width: 0; font-size: calc(var(--size) * 0.55); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .k-worldclock .diff { font-size: calc(var(--size) * 0.36); color: var(--muted); }
    .k-worldclock .time { font-size: var(--size); font-weight: 200; font-variant-numeric: tabular-nums; }
    .k-worldclock .row:first-child .time { color: var(--accent); }
    .k-worldclock .empty { color: var(--muted); font-size: 6cqmin; text-align: center; }
  `,
  mount(root, ctx) {
    const cities = Array.isArray(ctx.options.cities) ? ctx.options.cities : [];
    const rows = cities.map((c) => {
      const row = document.createElement('div');
      row.className = 'row';
      const name = document.createElement('span');
      name.className = 'name';
      name.textContent = c.name || ZONES.find(([z]) => z === c.zone)?.[1] || c.zone;
      const diff = document.createElement('span');
      diff.className = 'diff';
      const time = document.createElement('span');
      time.className = 'time';
      row.append(name, diff, time);
      return { c, row, diff, time };
    });
    // 도시가 많아지면 글자를 줄여 한 화면에 다 들어가게 한다.
    root.style.setProperty('--n', Math.max(rows.length, 1));
    if (rows.length) root.replaceChildren(...rows.map((r) => r.row));
    else {
      const empty = document.createElement('div');
      empty.className = 'empty';
      empty.textContent = '설정에서 도시를 추가해 주세요.';
      root.replaceChildren(empty);
    }

    function draw() {
      const now = new Date();
      let local;
      try { local = dayNumber(parts(Intl.DateTimeFormat().resolvedOptions().timeZone, now)); } catch { return; }
      for (const r of rows) {
        try {
          const p = parts(r.c.zone, now);
          const d = dayNumber(p) - local;
          r.time.textContent = `${p.hour === '24' ? '00' : p.hour}:${p.minute}`;
          r.diff.textContent = d < 0 ? '어제' : d > 0 ? '내일' : '오늘';
        } catch {
          r.time.textContent = '--:--';
          r.diff.textContent = '';
        }
      }
    }

    draw();
    const timer = setInterval(() => {
      if (!ctx.isPaused()) draw();
    }, 5000);
    return () => {
      clearInterval(timer);
      root.style.removeProperty('--n');
    };
  },
};
