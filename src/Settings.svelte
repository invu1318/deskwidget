<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open, confirm } from '@tauri-apps/plugin-dialog';
  import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart';

  const IMAGE_FILTER = [
    { name: '사진·동영상', extensions: ['gif', 'webp', 'png', 'apng', 'jpg', 'jpeg', 'mp4', 'webm'] },
  ];

  const KIND_LABEL = { web: '웹', image: '사진', slideshow: '슬라이드쇼', clock: '시계', memo: '메모' };
  const PLACEHOLDER = {
    web: 'https://example.com',
    image: '파일 경로 또는 이미지 URL',
    slideshow: '사진이 들어 있는 폴더 경로',
  };
  const REFRESH_OPTIONS = [0, 1, 5, 10, 30, 60];

  let widgets = $state([]);
  let editMode = $state(false);
  let autostart = $state(false);
  let kind = $state('web');
  let source = $state('');
  let error = $state('');

  async function refresh() {
    widgets = await invoke('get_widgets');
    editMode = await invoke('get_edit_mode');
  }

  async function run(task) {
    error = '';
    try {
      await task();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    run(async () => {
      await refresh();
      autostart = await isEnabled();
    });
    const unlisten = [
      listen('widgets-changed', refresh),
      listen('edit-mode-changed', (event) => (editMode = event.payload)),
    ];
    return () => unlisten.forEach((p) => p.then((off) => off()));
  });

  const save = (w) => run(() => invoke('update_widget', { widget: $state.snapshot(w) }));

  function set(w, key, value) {
    w[key] = value;
    save(w);
  }

  const add = () =>
    run(async () => {
      await invoke('add_widget', { kind, source: kind in PLACEHOLDER ? source : '' });
      source = '';
      await refresh();
    });

  const remove = (w) =>
    run(async () => {
      if (!(await confirm(`'${w.name}' 위젯을 삭제할까요?`, { title: '위젯 삭제', kind: 'warning' }))) return;
      await invoke('remove_widget', { id: w.id });
      await refresh();
    });

  const toggleEdit = () => run(() => invoke('set_edit_mode', { on: !editMode }));

  const toggleAutostart = () =>
    run(async () => {
      if (autostart) await disable();
      else await enable();
      autostart = await isEnabled();
    });

  async function pickImage(w) {
    const path = await open({ multiple: false, filters: IMAGE_FILTER });
    if (!path) return;
    if (w) set(w, 'source', path);
    else source = path;
  }

  async function pickFolder(w) {
    const path = await open({ directory: true });
    if (!path) return;
    if (w) set(w, 'source', path);
    else source = path;
  }

  const number = (event, min = 0) => Math.max(min, Math.round(Number(event.currentTarget.value)) || 0);
</script>

<main>
  <header>
    <h1>바탕화면 위젯</h1>
    <div class="actions">
      <label><input type="checkbox" checked={autostart} onchange={toggleAutostart} /> 부팅 시 실행</label>
      <button class:primary={editMode} onclick={toggleEdit}>{editMode ? '편집 끝내기' : '편집 모드'}</button>
    </div>
  </header>

  {#if editMode}
    <p class="hint">위젯을 끌어서 옮기고, 가장자리나 모서리를 끌어서 크기를 바꾸세요. '편집 끝내기'를 누르면 위치가 저장됩니다.</p>
  {/if}
  {#if error}
    <p class="error">{error}</p>
  {/if}

  <section class="add">
    <select bind:value={kind}>
      <option value="web">웹페이지</option>
      <option value="image">움직이는 사진·동영상</option>
      <option value="slideshow">사진 폴더 슬라이드쇼</option>
      <option value="clock">시계</option>
      <option value="memo">메모</option>
    </select>
    {#if kind in PLACEHOLDER}
      <input placeholder={PLACEHOLDER[kind]} bind:value={source} onkeydown={(e) => e.key === 'Enter' && add()} />
    {:else}
      <span class="grow note">{kind === 'clock' ? '현재 시각과 날짜를 보여줘요.' : '위젯에 바로 글을 적는 메모장이에요.'}</span>
    {/if}
    {#if kind === 'image'}
      <button onclick={() => pickImage()}>파일 선택</button>
    {:else if kind === 'slideshow'}
      <button onclick={() => pickFolder()}>폴더 선택</button>
    {/if}
    <button class="primary" onclick={add}>추가</button>
  </section>

  {#each widgets as w (w.id)}
    <article class:off={!w.visible}>
      <div class="row">
        <span class="badge">{KIND_LABEL[w.kind]}</span>
        <input class="grow" value={w.name} onchange={(e) => set(w, 'name', e.currentTarget.value)} />
        <label><input type="checkbox" checked={w.visible} onchange={(e) => set(w, 'visible', e.currentTarget.checked)} /> 표시</label>
        <button class="danger" onclick={() => remove(w)}>삭제</button>
      </div>
      {#if w.kind in PLACEHOLDER}
        <div class="row">
          <input class="grow" value={w.source} onchange={(e) => set(w, 'source', e.currentTarget.value)} />
          {#if w.kind === 'image'}
            <button onclick={() => pickImage(w)}>변경</button>
          {:else if w.kind === 'slideshow'}
            <button onclick={() => pickFolder(w)}>변경</button>
          {/if}
        </div>
      {/if}
      <div class="row">
        <label class="opacity">
          불투명도
          <input
            type="range"
            min="0.1"
            max="1"
            step="0.05"
            value={w.opacity}
            oninput={(e) => set(w, 'opacity', Number(e.currentTarget.value))}
          />
          <span class="num">{Math.round(w.opacity * 100)}%</span>
        </label>
        <select value={w.layer} onchange={(e) => set(w, 'layer', e.currentTarget.value)}>
          <option value="desktop">바탕화면에 고정</option>
          <option value="top">항상 위</option>
        </select>
        <label><input type="checkbox" checked={w.locked} onchange={(e) => set(w, 'locked', e.currentTarget.checked)} /> 위치 잠금</label>
        <label><input type="checkbox" checked={w.clickThrough} onchange={(e) => set(w, 'clickThrough', e.currentTarget.checked)} /> 클릭 통과</label>
        {#if w.kind === 'web'}
          <label><input type="checkbox" checked={w.mobile} onchange={(e) => set(w, 'mobile', e.currentTarget.checked)} /> 모바일 페이지</label>
        {/if}
      </div>
      <div class="row">
        <label>모서리 둥글기 <input class="short" type="number" min="0" max="500" value={w.radius} onchange={(e) => set(w, 'radius', number(e))} /> px</label>
        {#if w.kind === 'slideshow'}
          <label>사진 전환 <input class="short" type="number" min="1" value={w.slideSeconds} onchange={(e) => set(w, 'slideSeconds', number(e, 1))} /> 초마다</label>
        {/if}
        {#if w.kind === 'web'}
          <label>
            자동 새로고침
            <select value={w.refreshMinutes} onchange={(e) => set(w, 'refreshMinutes', Number(e.currentTarget.value))}>
              {#each REFRESH_OPTIONS as minutes}
                <option value={minutes}>{minutes ? `${minutes}분마다` : '끔'}</option>
              {/each}
            </select>
          </label>
        {/if}
      </div>
      {#if w.kind === 'web'}
        <div class="row">
          <label class="opacity">
            확대
            <input type="range" min="0.25" max="3" step="0.05" value={w.zoom} onchange={(e) => set(w, 'zoom', Number(e.currentTarget.value))} />
            <span class="num">{Math.round(w.zoom * 100)}%</span>
          </label>
          <label title="페이지를 이 폭으로 그려요. 0이면 위젯 폭에 맞춰요.">페이지 폭 <input class="short" type="number" min="0" step="10" value={w.pageWidth} onchange={(e) => set(w, 'pageWidth', number(e))} /></label>
          <label title="페이지를 왼쪽으로 이만큼 밀어요.">가로 이동 <input class="short" type="number" min="0" step="10" value={w.offsetX} onchange={(e) => set(w, 'offsetX', number(e))} /></label>
          <label title="페이지를 위로 이만큼 밀어요.">세로 이동 <input class="short" type="number" min="0" step="10" value={w.offsetY} onchange={(e) => set(w, 'offsetY', number(e))} /></label>
        </div>
      {/if}
    </article>
  {:else}
    <p class="empty">아직 위젯이 없어요. 위에서 웹페이지 주소나 사진을 추가해 보세요.</p>
  {/each}

  <p class="note">단축키: Ctrl+Alt+H 모든 위젯 숨기기/보이기 · Ctrl+Alt+C 클릭 통과 켜기/끄기 (트레이 메뉴에도 있어요)</p>
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: 'Segoe UI', 'Malgun Gothic', sans-serif;
    background: #f3f4f6;
    color: #111827;
  }
  main { padding: 20px 24px; display: flex; flex-direction: column; gap: 12px; }
  header { display: flex; align-items: center; justify-content: space-between; }
  h1 { font-size: 20px; margin: 0; }
  .actions, .row, .add { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .add, article { background: #fff; border: 1px solid #e5e7eb; border-radius: 10px; padding: 12px; }
  article { display: flex; flex-direction: column; gap: 10px; }
  article.off { opacity: 0.55; }
  .add input, .grow { flex: 1; min-width: 160px; }
  input, select, button { font: inherit; padding: 6px 10px; border: 1px solid #d1d5db; border-radius: 6px; background: #fff; }
  input[type='checkbox'], input[type='range'] { padding: 0; }
  button { cursor: pointer; }
  button:hover { background: #f9fafb; }
  .primary { background: #2563eb; border-color: #2563eb; color: #fff; }
  .primary:hover { background: #1d4ed8; }
  .danger { color: #dc2626; }
  .badge { font-size: 12px; padding: 2px 8px; border-radius: 999px; background: #dbeafe; color: #1e40af; }
  .opacity { display: flex; align-items: center; gap: 6px; }
  .num { width: 40px; text-align: right; }
  .short { width: 64px; }
  .note { margin: 0; color: #6b7280; font-size: 13px; }
  .hint { margin: 0; padding: 8px 12px; border-radius: 8px; background: #dbeafe; color: #1e3a8a; }
  .error { margin: 0; padding: 8px 12px; border-radius: 8px; background: #fee2e2; color: #991b1b; }
  .empty { color: #6b7280; text-align: center; padding: 32px 0; }
</style>
