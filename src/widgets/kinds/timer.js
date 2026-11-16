const pad = (n) => String(n).padStart(2, '0');

function beep() {
  try {
    const ac = new AudioContext();
    ac.resume();
    const gain = ac.createGain();
    gain.connect(ac.destination);
    [0, 0.25, 0.5].forEach((at) => {
      const osc = ac.createOscillator();
      osc.frequency.value = 880;
      osc.connect(gain);
      gain.gain.setValueAtTime(0.15, ac.currentTime + at);
      gain.gain.setValueAtTime(0, ac.currentTime + at + 0.15);
      osc.start(ac.currentTime + at);
      osc.stop(ac.currentTime + at + 0.16);
    });
    setTimeout(() => ac.close(), 1500);
  } catch {}
}

export default {
  kind: 'timer',
  label: '타이머',
  note: '집중과 휴식을 번갈아 알려 주는 뽀모도로 타이머예요.',
  defaults: { focusMinutes: 25, breakMinutes: 5, sound: true },
  fields: [
    { key: 'focusMinutes', label: '집중 시간(분)', type: 'number', min: 1, max: 180 },
    { key: 'breakMinutes', label: '휴식 시간(분)', type: 'number', min: 1, max: 60 },
    { key: 'sound', label: '끝나면 소리 내기', type: 'toggle' },
  ],
  css: `
    .k-timer { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3cqmin; padding: 6cqmin; }
    .k-timer .phase { font-size: 7cqmin; font-weight: 600; color: var(--muted); }
    .k-timer .phase.focus { color: var(--accent); }
    .k-timer .time { font-size: 28cqmin; font-weight: 200; line-height: 1; font-variant-numeric: tabular-nums; white-space: nowrap; }
    .k-timer .bar { width: 80%; height: 1.6cqmin; background: var(--raised); border-radius: 1cqmin; overflow: hidden; }
    .k-timer .bar i { display: block; height: 100%; width: 0; background: var(--accent); }
    .k-timer .btns { display: flex; gap: 3cqmin; }
    .k-timer button { font: inherit; font-size: 5.5cqmin; padding: 1.6cqmin 5cqmin; color: var(--text); background: var(--raised); border: 1px solid var(--line); border-radius: 6cqmin; cursor: pointer; }
    .k-timer button.main { background: var(--accent); border-color: var(--accent); color: var(--on-accent); font-weight: 600; }
  `,
  mount(root, ctx) {
    const mins = (v, d) => Math.max(1, Number(v) || d);
    const length = (phase) => mins(phase === 'focus' ? ctx.options.focusMinutes : ctx.options.breakMinutes, phase === 'focus' ? 25 : 5) * 60000;

    // phase, running, endAt(실행 중 종료 시각), left(멈춘 동안 남은 ms)
    const s = ctx.data?.phase ? { ...ctx.data } : { phase: 'focus', running: false, endAt: 0, left: length('focus') };
    const save = () => ctx.saveData({ ...s });

    const phaseEl = document.createElement('div');
    const time = document.createElement('div');
    time.className = 'time';
    const bar = document.createElement('div');
    bar.className = 'bar';
    const fill = document.createElement('i');
    bar.append(fill);
    const btns = document.createElement('div');
    btns.className = 'btns';
    const go = document.createElement('button');
    go.className = 'main';
    const reset = document.createElement('button');
    reset.textContent = '초기화';
    btns.append(go, reset);
    root.replaceChildren(phaseEl, time, bar, btns);

    function remaining() {
      if (!s.running) return s.left;
      // 페이지가 오래 멈춰 있었어도 지나간 단계를 따라잡는다.
      let guard = 0;
      while (s.endAt <= Date.now() && guard++ < 1000) {
        s.phase = s.phase === 'focus' ? 'break' : 'focus';
        s.endAt += length(s.phase);
        if (guard === 1 && ctx.options.sound) beep();
      }
      if (guard) save();
      return s.endAt - Date.now();
    }

    function draw() {
      const left = Math.max(0, remaining());
      const sec = Math.ceil(left / 1000);
      phaseEl.className = s.phase === 'focus' ? 'phase focus' : 'phase';
      phaseEl.textContent = s.phase === 'focus' ? '집중' : '휴식';
      time.textContent = `${pad(Math.floor(sec / 60))}:${pad(sec % 60)}`;
      fill.style.width = `${(1 - left / length(s.phase)) * 100}%`;
      go.textContent = s.running ? '일시정지' : '시작';
    }

    go.onclick = () => {
      if (s.running) {
        s.left = Math.max(0, remaining());
        s.running = false;
      } else {
        s.endAt = Date.now() + s.left;
        s.running = true;
      }
      save();
      draw();
    };
    reset.onclick = () => {
      Object.assign(s, { phase: 'focus', running: false, endAt: 0, left: length('focus') });
      save();
      draw();
    };

    draw();
    const timer = setInterval(() => {
      if (s.running && !ctx.isPaused()) draw();
    }, 250);
    return () => clearInterval(timer);
  },
};
