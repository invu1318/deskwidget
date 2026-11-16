import { invoke } from '@tauri-apps/api/core';

const GB = 1024 ** 3;
const gb = (bytes) => (bytes / GB).toFixed(bytes >= 100 * GB ? 0 : 1);

export default {
  kind: 'system',
  label: 'PC 상태',
  note: 'CPU, 메모리, 디스크 사용량을 실시간으로 보여줘요.',
  defaults: {},
  fields: [],
  css: `
    .k-system { display: flex; flex-direction: column; justify-content: space-evenly; padding: 6cqmin 8cqmin; }
    .k-system .row { display: flex; flex-direction: column; gap: 1.6cqmin; }
    .k-system .head { display: flex; align-items: baseline; justify-content: space-between; gap: 3cqmin; }
    .k-system .name { color: var(--muted); font-size: 5cqmin; }
    .k-system .val { font-size: 9cqmin; font-weight: 200; line-height: 1; font-variant-numeric: tabular-nums; }
    .k-system .sub { font-size: 4.6cqmin; color: var(--muted); font-variant-numeric: tabular-nums; }
    .k-system .bar { height: 1.4cqmin; min-height: 2px; background: var(--raised); border-radius: 99px; overflow: hidden; }
    .k-system .fill { height: 100%; width: 0; background: var(--accent); border-radius: 99px; transition: width 0.6s ease; }
    .k-system .err { flex: 1; display: flex; align-items: center; justify-content: center; text-align: center; color: var(--muted); font-size: 5.6cqmin; }
  `,
  mount(root, ctx) {
    let alive = true;
    const rows = ['CPU', '메모리', '디스크'].map((label) => {
      const row = document.createElement('div');
      row.className = 'row';
      row.innerHTML = '<div class="head"><span class="name"></span><span class="sub"></span><span class="val"></span></div><div class="bar"><div class="fill"></div></div>';
      row.querySelector('.name').textContent = label;
      return { row, sub: row.querySelector('.sub'), val: row.querySelector('.val'), fill: row.querySelector('.fill') };
    });
    root.replaceChildren(...rows.map((r) => r.row));

    function set(r, percent, sub) {
      const p = Math.max(0, Math.min(100, percent));
      r.val.textContent = `${Math.round(p)}%`;
      r.sub.textContent = sub;
      r.fill.style.width = `${p}%`;
    }

    // 읽지 못한 항목은 그 줄만 오류로 보여준다.
    function fail(r) {
      r.val.textContent = '—';
      r.sub.textContent = '읽지 못했어요';
      r.fill.style.width = '0';
    }

    function setUsage(r, usage) {
      if (!usage?.total) return fail(r);
      set(r, (usage.used / usage.total) * 100, `${gb(usage.used)} / ${gb(usage.total)} GB`);
    }

    let busy = false;
    async function tick() {
      if (busy || ctx.isPaused()) return;
      busy = true;
      try {
        const s = await invoke('get_system_stats');
        if (!alive) return;
        if (!rows[0].row.isConnected) root.replaceChildren(...rows.map((r) => r.row));
        if (typeof s.cpu === 'number') set(rows[0], s.cpu, '');
        else fail(rows[0]);
        setUsage(rows[1], s.memory);
        setUsage(rows[2], s.disk);
      } catch {
        if (!alive) return;
        const err = document.createElement('div');
        err.className = 'err';
        err.textContent = '상태를 읽지 못했어요. 앱을 다시 시작해 주세요.';
        root.replaceChildren(err);
      } finally {
        busy = false;
      }
    }

    tick();
    const timer = setInterval(tick, 2000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  },
};
