// 편집 모드 오버레이: 이 창을 끌거나 크기를 바꾸면 Rust 쪽에서 위젯 창이 따라온다.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

const id = new URLSearchParams(location.search).get('id');
const win = getCurrentWindow();
let locked = false;

async function load() {
  const widget = await invoke('get_widget', { id });
  if (!widget) return;
  locked = widget.locked;
  document.body.classList.toggle('locked', locked);
  document.getElementById('name').textContent = (locked ? '🔒 ' : '') + widget.name;
}

document.getElementById('frame').addEventListener('mousedown', (event) => {
  if (event.button === 0 && !locked && !event.target.dataset.dir) win.startDragging();
});

for (const handle of document.querySelectorAll('[data-dir]')) {
  handle.addEventListener('mousedown', (event) => {
    if (event.button !== 0 || locked) return;
    event.stopPropagation();
    win.startResizeDragging(handle.dataset.dir);
  });
}

listen('widget-updated', (event) => {
  if (event.payload === id) load();
});
load();
