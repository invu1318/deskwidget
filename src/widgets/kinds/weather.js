const DAYS = ['일', '월', '화', '수', '목', '금', '토'];
const REFRESH_MS = 20 * 60 * 1000;

// WMO 날씨 코드 → [한국어 설명, 기호]
const CONDITIONS = [
  [[0], '맑음', '☀'],
  [[1], '대체로 맑음', '🌤'],
  [[2], '구름 많음', '⛅'],
  [[3], '흐림', '☁'],
  [[45, 48], '안개', '🌫'],
  [[51, 53, 55, 56, 57], '이슬비', '🌦'],
  [[61, 63, 65, 66, 67, 80, 81, 82], '비', '🌧'],
  [[71, 73, 75, 77, 85, 86], '눈', '🌨'],
  [[95, 96, 99], '뇌우', '⛈'],
];
const condition = (code) => CONDITIONS.find(([codes]) => codes.includes(code)) ?? [[], '알 수 없음', '·'];

async function getJson(url) {
  const res = await fetch(url, { signal: AbortSignal.timeout(15000) });
  if (!res.ok) throw new Error('http');
  return res.json();
}

async function locate(city) {
  // 한글 지명은 '서울특별시', '수원시'처럼 행정 단위가 붙은 이름으로만 등록된 곳이 많고,
  // 짧은 이름은 같은 이름의 작은 마을이 먼저 나온다. 여러 이름으로 찾아 인구가 가장 많은 곳을 고른다.
  const names = /^[가-힣]+$/.test(city) && !/[시군도]$/.test(city) ? [city, `${city}특별시`, `${city}광역시`, `${city}시`, `${city}군`] : [city];
  const found = await Promise.allSettled(
    names.map((name) => getJson(`https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(name)}&count=5&language=ko&format=json`)),
  );
  const answered = found.filter((result) => result.status === 'fulfilled').map((result) => result.value);
  if (!answered.length) throw new Error('network');
  const hit = answered.flatMap((json) => json.results ?? []).sort((a, b) => (b.population ?? 0) - (a.population ?? 0))[0];
  if (!hit) {
    // 일부 조회가 실패했다면 도시가 없는 것인지 알 수 없으므로 네트워크 오류로 본다.
    if (answered.length < names.length) throw new Error('network');
    return null;
  }
  return { name: hit.name, latitude: hit.latitude, longitude: hit.longitude };
}

async function forecast({ latitude, longitude }) {
  const json = await getJson(
    `https://api.open-meteo.com/v1/forecast?latitude=${latitude}&longitude=${longitude}` +
      '&current=temperature_2m,weather_code&daily=weather_code,temperature_2m_max,temperature_2m_min&timezone=auto&forecast_days=5',
  );
  return { current: json.current, daily: json.daily };
}

export default {
  kind: 'weather',
  label: '날씨',
  note: '도시의 현재 날씨와 며칠간의 예보를 보여줘요.',
  defaults: { city: '서울' },
  fields: [{ key: 'city', label: '도시', type: 'text', placeholder: '서울', hint: '도시 이름을 입력하세요.' }],
  css: `
    .k-weather { display: flex; flex-direction: column; padding: 6cqmin 7cqmin; gap: 3cqmin; overflow: hidden; }
    .k-weather .place { font-size: 6cqmin; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .k-weather .now { display: flex; align-items: center; gap: 4cqmin; min-height: 0; }
    .k-weather .temp { font-size: 22cqmin; font-weight: 200; line-height: 1; font-variant-numeric: tabular-nums; }
    .k-weather .cond { font-size: 6.4cqmin; line-height: 1.3; }
    .k-weather .cond small { display: block; font-size: 5cqmin; color: var(--muted); font-variant-numeric: tabular-nums; }
    .k-weather .glyph { font-size: 12cqmin; margin-left: auto; line-height: 1; }
    .k-weather .days { flex: 1; min-height: 0; display: grid; grid-auto-flow: column; grid-auto-columns: 1fr; border-top: 1px solid var(--line); padding-top: 3cqmin; }
    .k-weather .day { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1cqmin; font-size: 4.6cqmin; font-variant-numeric: tabular-nums; min-width: 0; }
    .k-weather .day .dow { color: var(--muted); }
    .k-weather .day .g { font-size: 7cqmin; line-height: 1.1; }
    .k-weather .day .lo { color: var(--muted); }
    .k-weather .msg { flex: 1; display: flex; align-items: center; justify-content: center; text-align: center; color: var(--muted); font-size: 5.6cqmin; line-height: 1.5; }
    .k-weather .stale { color: var(--danger); font-size: 4cqmin; }
  `,
  mount(root, ctx) {
    const city = String(ctx.options.city ?? '').trim();
    let alive = true;
    let busy = false;
    let state = ctx.data?.city === city ? ctx.data : { city };
    let failed = false;

    function message(text) {
      const msg = document.createElement('div');
      msg.className = 'msg';
      msg.textContent = text;
      root.replaceChildren(msg);
    }

    function draw() {
      const { geo, weather } = state;
      if (!weather) return;
      const [, text, glyph] = condition(weather.current.weather_code);
      const d = weather.daily;
      const place = document.createElement('div');
      place.className = 'place';
      place.textContent = geo.name;
      if (failed) {
        const stale = document.createElement('span');
        stale.className = 'stale';
        stale.textContent = '  · 새로고침 실패';
        place.append(stale);
      }
      const now = document.createElement('div');
      now.className = 'now';
      now.innerHTML = '<div class="temp"></div><div class="cond"><div class="t"></div><small></small></div><div class="glyph"></div>';
      now.querySelector('.temp').textContent = `${Math.round(weather.current.temperature_2m)}°`;
      now.querySelector('.t').textContent = text;
      now.querySelector('small').textContent = `최고 ${Math.round(d.temperature_2m_max[0])}° · 최저 ${Math.round(d.temperature_2m_min[0])}°`;
      now.querySelector('.glyph').textContent = glyph;
      const days = document.createElement('div');
      days.className = 'days';
      for (let i = 1; i < d.time.length; i++) {
        const day = document.createElement('div');
        day.className = 'day';
        const dow = DAYS[new Date(`${d.time[i]}T00:00:00`).getDay()];
        day.innerHTML = '<span class="dow"></span><span class="g"></span><span class="hi"></span><span class="lo"></span>';
        day.querySelector('.dow').textContent = dow;
        day.querySelector('.g').textContent = condition(d.weather_code[i])[2];
        day.querySelector('.hi').textContent = `${Math.round(d.temperature_2m_max[i])}°`;
        day.querySelector('.lo').textContent = `${Math.round(d.temperature_2m_min[i])}°`;
        days.append(day);
      }
      root.replaceChildren(place, now, days);
    }

    async function refresh() {
      if (busy || !alive) return;
      busy = true;
      try {
        if (!state.geo) {
          const geo = await locate(city);
          if (!alive) return;
          if (!geo) {
            message(`'${city}'을(를) 찾을 수 없어요.\n설정에서 도시 이름을 확인해 주세요.`);
            root.firstChild.style.whiteSpace = 'pre-line';
            return;
          }
          state = { city, geo };
        }
        const weather = await forecast(state.geo);
        if (!alive) return;
        state = { ...state, weather };
        failed = false;
        ctx.saveData(state);
        draw();
      } catch {
        if (!alive) return;
        failed = true;
        if (state.weather) draw();
        else {
          message('날씨를 불러오지 못했어요.\n인터넷 연결을 확인해 주세요.');
          root.firstChild.style.whiteSpace = 'pre-line';
        }
      } finally {
        busy = false;
      }
    }

    if (!city) {
      message('설정에서 도시를 입력해 주세요.');
      return () => {};
    }
    if (state.weather) draw();
    else message('불러오는 중…');
    refresh();
    const timer = setInterval(() => {
      if (!ctx.isPaused()) refresh();
    }, REFRESH_MS);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  },
};
