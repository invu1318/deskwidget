import { invoke } from '@tauri-apps/api/core';

const ICONS = {
  prev: '<path d="M6 5h2v14H6zM20 5v14L9 12z"/>',
  next: '<path d="M16 5h2v14h-2zM4 5l11 7L4 19z"/>',
  play: '<path d="M7 4l13 8-13 8z"/>',
  pause: '<path d="M6 4h4v16H6zM14 4h4v16h-4z"/>',
};
const svg = (name) => `<svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">${ICONS[name]}</svg>`;

export default {
  kind: 'media',
  label: '지금 재생 중',
  note: '재생 중인 곡을 보여주고 이전·재생·다음을 조작해요.',
  defaults: {},
  fields: [],
  css: `
    .k-media { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 5cqmin; padding: 6cqmin 8cqmin; text-align: center; }
    .k-media .info { width: 100%; min-width: 0; }
    .k-media .title { font-size: 8cqmin; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .k-media .artist { font-size: 5.4cqmin; color: var(--muted); margin-top: 1cqmin; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .k-media .ctl { display: flex; align-items: center; gap: 6cqmin; }
    .k-media button { display: flex; align-items: center; justify-content: center; border: 0; background: none; color: var(--text); cursor: pointer; width: 12cqmin; height: 12cqmin; border-radius: 50%; padding: 0; }
    .k-media button svg { width: 55%; height: 55%; }
    .k-media button:hover { background: var(--raised); }
    .k-media .pp { background: var(--accent); color: var(--on-accent); width: 16cqmin; height: 16cqmin; }
    .k-media .pp:hover { background: var(--accent); filter: brightness(1.1); }
    .k-media .idle { color: var(--muted); font-size: 5.6cqmin; }
    .k-media.none .ctl { display: none; }
  `,
  mount(root, ctx) {
    let alive = true;
    root.classList.add('none');
    root.innerHTML = '<div class="info"><div class="title"></div><div class="artist"></div></div><div class="ctl"><button data-a="prev" aria-label="이전">' +
      svg('prev') + '</button><button class="pp" data-a="toggle" aria-label="재생/일시정지"></button><button data-a="next" aria-label="다음">' + svg('next') + '</button></div>';
    const title = root.querySelector('.title');
    const artist = root.querySelector('.artist');
    const pp = root.querySelector('.pp');
    pp.innerHTML = svg('play');
    title.className = 'title idle';
    title.textContent = '재생 중인 곡이 없어요';

    function show(info) {
      root.classList.toggle('none', !info.active);
      if (!info.active) {
        title.className = 'title idle';
        title.textContent = '재생 중인 곡이 없어요';
        artist.textContent = '';
        return;
      }
      title.className = 'title';
      title.textContent = info.title || '제목 없음';
      artist.textContent = info.artist;
      pp.innerHTML = svg(info.playing ? 'pause' : 'play');
    }

    // 요청은 항상 하나씩만 보낸다. 응답이 늦어도 요청이 쌓이지 않도록, 대기 중인 일은 표시만 해 두고 순서대로 처리한다.
    let busy = false;
    let pendingAction = null;
    let wantPoll = false;
    const timers = new Set();

    function later(fn, ms) {
      const t = setTimeout(() => {
        timers.delete(t);
        fn();
      }, ms);
      timers.add(t);
    }

    async function pump() {
      if (busy) return;
      busy = true;
      try {
        while (alive && (pendingAction || wantPoll)) {
          if (pendingAction) {
            const action = pendingAction;
            pendingAction = null;
            try {
              await invoke('media_control', { action });
            } catch {
              // 조작할 수 없는 상태면 다음 갱신에서 화면이 정리된다.
            }
            if (alive) later(() => requestPoll(), 300);
          } else {
            wantPoll = false;
            try {
              const info = await invoke('get_media_info');
              if (alive) show(info);
            } catch {
              if (alive) show({ active: false });
            }
          }
        }
      } finally {
        busy = false;
      }
    }

    function requestPoll() {
      if (!alive || ctx.isPaused()) return;
      wantPoll = true;
      pump();
    }

    function onClick(e) {
      const btn = e.target.closest('button');
      if (!btn || !root.contains(btn) || !alive) return;
      pendingAction = btn.dataset.a;
      pump();
    }
    root.addEventListener('click', onClick);

    requestPoll();
    const timer = setInterval(requestPoll, 1500);
    return () => {
      alive = false;
      clearInterval(timer);
      timers.forEach(clearTimeout);
      root.removeEventListener('click', onClick);
    };
  },
};
