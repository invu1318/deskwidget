<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { open, confirm } from '@tauri-apps/plugin-dialog';
  import { isEnabled, enable, disable } from '@tauri-apps/plugin-autostart';
  import { kinds, optionsOf } from './widgets/index.js';

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

  const KIND_CHOICES = [
    ['web', '웹페이지'],
    ['image', '사진·동영상'],
    ['slideshow', '슬라이드쇼'],
    ['clock', '시계'],
    ['memo', '메모'],
    ...Object.values(kinds).map((k) => [k.kind, k.label]),
  ];
  const kindLabel = (k) => KIND_LABEL[k] ?? kinds[k]?.label ?? k;

  // 패널 위젯의 설정값(options)을 고친다.
  const setOption = (w, key, value) => set(w, 'options', { ...optionsOf(w), [key]: value });
  const setItem = (w, f, index, key, value) =>
    setOption(w, f.key, optionsOf(w)[f.key].map((item, i) => (i === index ? { ...item, [key]: value } : item)));
  const removeItem = (w, f, index) => setOption(w, f.key, optionsOf(w)[f.key].filter((_, i) => i !== index));
  const addItem = (w, f) =>
    setOption(w, f.key, [...(optionsOf(w)[f.key] ?? []), Object.fromEntries(f.item.map((sub) => [sub.key, sub.default ?? '']))]);

  async function pickAny(onchange) {
    const path = await open({ multiple: false });
    if (path) onchange(path);
  }
  const KIND_NOTE = {
    web: '웹페이지 주소를 넣으면 그 페이지가 위젯으로 떠요.',
    image: 'GIF·WebP·APNG 사진이나 mp4·webm 동영상을 소리 없이 반복해요.',
    slideshow: '폴더 안의 사진을 차례로 보여줘요.',
    clock: '현재 시각과 날짜를 보여줘요.',
    memo: '위젯에 바로 글을 적는 메모장이에요.',
  };
  const SOURCE_LABEL = { web: '주소', image: '파일', slideshow: '폴더' };

  let selectedId = $state(null);
  let adding = $state(false);
  const current = $derived(widgets.find((w) => w.id === selectedId) ?? widgets[0]);
  const showAdd = $derived(adding || !current);

  function select(w) {
    selectedId = w.id;
    adding = false;
  }

  const addAndSelect = () =>
    run(async () => {
      const before = new Set(widgets.map((w) => w.id));
      await invoke('add_widget', { kind, source: kind in PLACEHOLDER ? source : '' });
      source = '';
      await refresh();
      const made = widgets.find((w) => !before.has(w.id));
      if (made) select(made);
    });
</script>

{#snippet toggle(checked, label, onchange)}
  <button class="sw" role="switch" aria-checked={checked} aria-label={label} onclick={() => onchange(!checked)}></button>
{/snippet}

{#snippet control(f, value, onchange)}
  {#if f.type === 'toggle'}
    {@render toggle(!!value, f.label, onchange)}
  {:else if f.type === 'select'}
    <select class="field" aria-label={f.label} {value} onchange={(e) => onchange(e.currentTarget.value)}>
      {#each f.options as [v, text]}
        <option value={v}>{text}</option>
      {/each}
    </select>
  {:else if f.type === 'number'}
    <input class="field short" type="number" min={f.min} max={f.max} aria-label={f.label} {value} onchange={(e) => onchange(Number(e.currentTarget.value) || 0)} />
  {:else if f.type === 'date'}
    <input class="field" type="date" aria-label={f.label} {value} onchange={(e) => onchange(e.currentTarget.value)} />
  {:else}
    <input class="field grow" placeholder={f.placeholder ?? ''} aria-label={f.label} {value} onchange={(e) => onchange(e.currentTarget.value)} />
    {#if f.type === 'file'}
      <button class="btn" onclick={() => pickAny(onchange)}>찾기</button>
    {/if}
  {/if}
{/snippet}

<div class="app">
  <nav class="side" aria-label="위젯 목록">
    <h1>DeskWidget</h1>
    <div class="items">
      {#each widgets as w (w.id)}
        <button class="item" class:off={!w.visible} aria-current={!showAdd && current?.id === w.id} onclick={() => select(w)}>
          <span class="dot"></span><span class="n">{w.name}</span><span class="kind">{kindLabel(w.kind)}</span>
        </button>
      {/each}
      <button class="btn add" aria-current={showAdd} onclick={() => (adding = true)}>＋ 위젯 추가</button>
    </div>
    <div class="foot">
      <div class="line"><span>부팅 시 실행</span>{@render toggle(autostart, '부팅 시 실행', toggleAutostart)}</div>
      <p class="keys">Ctrl+Alt+H 모두 숨기기<br />Ctrl+Alt+C 클릭 통과</p>
    </div>
  </nav>

  <section class="main">
    {#if editMode}
      <p class="hint">위젯을 끌어서 옮기고, 가장자리나 모서리를 끌어서 크기를 바꾸세요. '편집 끝내기'를 누르면 위치가 저장됩니다.</p>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}

    {#if showAdd}
      <div class="head">
        <h2 class="name">새 위젯</h2>
        <button class="btn primary" onclick={toggleEdit}>{editMode ? '편집 끝내기' : '편집 모드'}</button>
      </div>
      {#if !widgets.length}
        <p class="note">아직 위젯이 없어요. 종류를 고르고 추가해 보세요.</p>
      {/if}
      <div class="group">
        <h3>종류</h3>
        <div class="kinds">
          {#each KIND_CHOICES as [value, label]}
            <button aria-pressed={kind === value} onclick={() => (kind = value)}>{label}</button>
          {/each}
        </div>
        <p class="note">{KIND_NOTE[kind] ?? kinds[kind]?.note}</p>
      </div>
      {#if kind in PLACEHOLDER}
        <div class="group">
          <h3>원본</h3>
          <div class="rows">
            <div class="r">
              <span class="l">{SOURCE_LABEL[kind]}</span>
              <input class="field grow" placeholder={PLACEHOLDER[kind]} aria-label={SOURCE_LABEL[kind]} bind:value={source} onkeydown={(e) => e.key === 'Enter' && addAndSelect()} />
              {#if kind === 'image'}
                <button class="btn" onclick={() => pickImage()}>파일 선택</button>
              {:else if kind === 'slideshow'}
                <button class="btn" onclick={() => pickFolder()}>폴더 선택</button>
              {/if}
            </div>
          </div>
        </div>
      {/if}
      <div><button class="btn primary" onclick={addAndSelect}>위젯 추가</button></div>
    {:else}
      {@const w = current}
      <div class="head">
        <input class="field name" value={w.name} aria-label="위젯 이름" onchange={(e) => set(w, 'name', e.currentTarget.value)} />
        <button class="btn ghost" onclick={() => remove(w)}>삭제</button>
        <button class="btn primary" onclick={toggleEdit}>{editMode ? '편집 끝내기' : '편집 모드'}</button>
      </div>

      <div class="group">
        <h3>원본</h3>
        <div class="rows">
          {#if w.kind in PLACEHOLDER}
            <div class="r">
              <span class="l">{SOURCE_LABEL[w.kind]}</span>
              <input class="field grow" value={w.source} aria-label={SOURCE_LABEL[w.kind]} onchange={(e) => set(w, 'source', e.currentTarget.value)} />
              {#if w.kind === 'image'}
                <button class="btn" onclick={() => pickImage(w)}>변경</button>
              {:else if w.kind === 'slideshow'}
                <button class="btn" onclick={() => pickFolder(w)}>변경</button>
              {/if}
            </div>
          {/if}
          <div class="r">
            <span class="l">표시</span><span class="grow"><small>끄면 바탕화면에서 숨겨요</small></span>
            {@render toggle(w.visible, '표시', (v) => set(w, 'visible', v))}
          </div>
        </div>
      </div>

      {#if kinds[w.kind]?.fields?.length}
        <div class="group">
          <h3>{kinds[w.kind].label} 설정</h3>
          <div class="rows">
            {#each kinds[w.kind].fields as f}
              {#if f.type === 'list'}
                <div class="r top">
                  <span class="l">{f.label}</span>
                  <div class="list">
                    {#each optionsOf(w)[f.key] ?? [] as item, i}
                      <div class="li">
                        {#each f.item as sub}
                          {@render control(sub, item[sub.key], (v) => setItem(w, f, i, sub.key, v))}
                        {/each}
                        <button class="btn ghost" onclick={() => removeItem(w, f, i)}>삭제</button>
                      </div>
                    {/each}
                    <div><button class="btn" onclick={() => addItem(w, f)}>＋ {f.addLabel ?? '추가'}</button></div>
                  </div>
                </div>
              {:else}
                <div class="r">
                  <span class="l">{f.label}</span>
                  <span class="ctl">
                    {#if f.hint}<small>{f.hint}</small>{/if}
                    {@render control(f, optionsOf(w)[f.key], (v) => setOption(w, f.key, v))}
                  </span>
                </div>
              {/if}
            {/each}
          </div>
        </div>
      {/if}

      <div class="group">
        <h3>모양</h3>
        <div class="rows">
          <div class="r">
            <span class="l">불투명도</span>
            <input type="range" min="0.1" max="1" step="0.05" value={w.opacity} aria-label="불투명도" oninput={(e) => set(w, 'opacity', Number(e.currentTarget.value))} />
            <span class="num">{Math.round(w.opacity * 100)}%</span>
          </div>
          <div class="r">
            <span class="l">모서리 둥글기</span>
            <input type="range" min="0" max="300" step="2" value={w.radius} aria-label="모서리 둥글기" oninput={(e) => set(w, 'radius', number(e))} />
            <span class="num">{w.radius}px</span>
          </div>
        </div>
      </div>

      <div class="group">
        <h3>동작</h3>
        <div class="rows">
          <div class="r">
            <span class="l">놓이는 곳</span><span class="grow"></span>
            <div class="seg">
              <button aria-pressed={w.layer === 'desktop'} onclick={() => set(w, 'layer', 'desktop')}>바탕화면에 고정</button>
              <button aria-pressed={w.layer === 'top'} onclick={() => set(w, 'layer', 'top')}>항상 위</button>
            </div>
          </div>
          <div class="r">
            <span class="l">위치 잠금</span><span class="grow"></span>
            {@render toggle(w.locked, '위치 잠금', (v) => set(w, 'locked', v))}
          </div>
          <div class="r">
            <span class="l">클릭 통과</span><span class="grow"><small>위젯 너머의 바탕화면을 클릭해요</small></span>
            {@render toggle(w.clickThrough, '클릭 통과', (v) => set(w, 'clickThrough', v))}
          </div>
          {#if w.kind === 'slideshow'}
            <div class="r">
              <span class="l">사진 전환</span><span class="grow"></span>
              <label class="unit"><input class="field short" type="number" min="1" value={w.slideSeconds} onchange={(e) => set(w, 'slideSeconds', number(e, 1))} /> 초마다</label>
            </div>
          {/if}
        </div>
      </div>

      {#if w.kind === 'web'}
        <div class="group">
          <h3>웹페이지 보기</h3>
          <div class="rows">
            <div class="r">
              <span class="l">확대</span>
              <input type="range" min="0.25" max="3" step="0.05" value={w.zoom} aria-label="확대" onchange={(e) => set(w, 'zoom', Number(e.currentTarget.value))} />
              <span class="num">{Math.round(w.zoom * 100)}%</span>
            </div>
            <div class="r">
              <span class="l">잘라 보기</span>
              <div class="tri">
                <label title="페이지를 이 폭으로 그려요. 0이면 위젯 폭에 맞춰요.">폭 <input class="field short" type="number" min="0" step="10" value={w.pageWidth} onchange={(e) => set(w, 'pageWidth', number(e))} /></label>
                <label title="페이지를 왼쪽으로 이만큼 밀어요.">가로 <input class="field short" type="number" min="0" step="10" value={w.offsetX} onchange={(e) => set(w, 'offsetX', number(e))} /></label>
                <label title="페이지를 위로 이만큼 밀어요.">세로 <input class="field short" type="number" min="0" step="10" value={w.offsetY} onchange={(e) => set(w, 'offsetY', number(e))} /></label>
              </div>
            </div>
            <div class="r">
              <span class="l">자동 새로고침</span><span class="grow"></span>
              <select class="field" aria-label="자동 새로고침" value={w.refreshMinutes} onchange={(e) => set(w, 'refreshMinutes', Number(e.currentTarget.value))}>
                {#each REFRESH_OPTIONS as minutes}
                  <option value={minutes}>{minutes ? `${minutes}분마다` : '끔'}</option>
                {/each}
              </select>
            </div>
            <div class="r">
              <span class="l">모바일 페이지</span><span class="grow"></span>
              {@render toggle(w.mobile, '모바일 페이지', (v) => set(w, 'mobile', v))}
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </section>
</div>

<style>
  :global(:root) {
    color-scheme: dark;
    --accent: #f5a524;
    --on-accent: #1a1300;
    --bg: #14161a;
    --panel: #1c1f25;
    --raised: #262a33;
    --line: #343945;
    --text: #eef0f3;
    --muted: #a9b0bc;
    --danger: #ff8a80;
  }
  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
    font: 14px/1.45 'Segoe UI Variable Text', 'Segoe UI', 'Malgun Gothic', sans-serif;
  }
  .app { height: 100vh; display: grid; grid-template-columns: 240px 1fr; }
  button, input, select { font: inherit; color: inherit; }
  button { cursor: pointer; }
  :focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }

  .btn { height: 32px; padding: 0 14px; border-radius: 7px; border: 1px solid var(--line); background: var(--raised); flex: none; }
  .btn:hover { border-color: var(--muted); }
  .btn.primary { background: var(--accent); border-color: var(--accent); color: var(--on-accent); font-weight: 600; }
  .btn.ghost { background: none; border-color: transparent; color: var(--danger); }
  .field { height: 32px; padding: 0 10px; border-radius: 7px; border: 1px solid var(--line); background: var(--bg); min-width: 0; }
  .short { width: 72px; }
  .sw { width: 42px; height: 24px; border-radius: 12px; border: 1px solid var(--line); background: var(--raised); position: relative; padding: 0; flex: none; }
  .sw::after { content: ''; position: absolute; top: 3px; left: 3px; width: 16px; height: 16px; border-radius: 50%; background: var(--muted); transition: left 0.12s; }
  .sw[aria-checked='true'] { background: var(--accent); border-color: var(--accent); }
  .sw[aria-checked='true']::after { left: 21px; background: var(--on-accent); }
  input[type='range'] { accent-color: var(--accent); flex: 1; min-width: 0; height: 24px; margin: 0; }
  .num { width: 52px; text-align: right; font-variant-numeric: tabular-nums; color: var(--muted); }
  .kind { font-size: 11px; color: var(--muted); border: 1px solid var(--line); border-radius: 5px; padding: 1px 6px; flex: none; }

  .side { border-right: 1px solid var(--line); display: flex; flex-direction: column; padding: 14px 10px 12px; min-height: 0; }
  .side h1 { font-size: 15px; margin: 0 6px 10px; font-weight: 600; }
  .items { display: flex; flex-direction: column; gap: 4px; overflow-y: auto; min-height: 0; }
  .item { display: flex; align-items: center; gap: 8px; height: 40px; padding: 0 10px; border-radius: 7px; border: 0; background: none; text-align: left; width: 100%; flex: none; }
  .item:hover { background: var(--raised); }
  .item[aria-current='true'] { background: var(--raised); font-weight: 600; }
  .item .n { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item.off .n { color: var(--muted); }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); flex: none; }
  .item.off .dot { background: none; border: 1px solid var(--muted); }
  .add { margin-top: 8px; border-style: dashed; background: none; color: var(--muted); }
  .add[aria-current='true'] { border-color: var(--accent); color: var(--text); }
  .foot { margin-top: auto; border-top: 1px solid var(--line); padding: 12px 6px 0; display: flex; flex-direction: column; gap: 10px; font-size: 13px; }
  .line { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .keys { color: var(--muted); font-size: 12px; margin: 0; }

  .main { padding: 18px 24px; overflow-y: auto; display: flex; flex-direction: column; gap: 18px; min-width: 0; }
  .head { display: flex; align-items: center; gap: 10px; }
  .name { flex: 1; font-size: 22px; font-weight: 600; height: 40px; margin: 0 0 0 -6px; padding: 0 6px; }
  input.name { background: none; border-color: transparent; }
  input.name:hover { border-color: var(--line); }
  h2.name { line-height: 40px; }
  h3 { font-size: 12px; font-weight: 600; color: var(--muted); margin: 0 0 8px; }
  .rows { background: var(--panel); border: 1px solid var(--line); border-radius: 10px; }
  .r { display: flex; align-items: center; gap: 12px; min-height: 46px; padding: 6px 14px; }
  .r + .r { border-top: 1px solid var(--line); }
  .l { width: 112px; flex: none; }
  .grow { flex: 1; min-width: 0; }
  small { color: var(--muted); font-size: 12px; }
  .seg { display: flex; border: 1px solid var(--line); border-radius: 7px; overflow: hidden; }
  .seg button { height: 30px; padding: 0 12px; border: 0; background: none; }
  .seg button[aria-pressed='true'] { background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .kinds { display: grid; grid-template-columns: repeat(auto-fill, minmax(112px, 1fr)); gap: 8px; }
  .kinds button { height: 36px; border: 1px solid var(--line); border-radius: 7px; background: var(--panel); }
  .kinds button:hover { border-color: var(--muted); }
  .kinds button[aria-pressed='true'] { background: var(--accent); border-color: var(--accent); color: var(--on-accent); font-weight: 600; }
  .ctl { flex: 1; min-width: 0; display: flex; align-items: center; justify-content: flex-end; gap: 10px; }
  .ctl small { flex: 1; }
  .r.top { align-items: flex-start; }
  .r.top .l { line-height: 32px; }
  .list { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 8px; }
  .li { display: flex; align-items: center; gap: 8px; }
  .tri { display: flex; gap: 14px; flex: 1; flex-wrap: wrap; }
  .tri label, .unit { display: flex; align-items: center; gap: 6px; color: var(--muted); font-size: 13px; }
  .tri .field, .unit .field { color: var(--text); }
  .note { margin: 8px 0 0; color: var(--muted); font-size: 13px; }
  .hint { margin: 0; padding: 8px 12px; border-radius: 8px; border: 1px solid var(--accent); }
  .error { margin: 0; padding: 8px 12px; border-radius: 8px; border: 1px solid var(--danger); color: var(--danger); }
</style>
