// 움직이는 사진 위젯: 로컬 파일은 asset 프로토콜로, URL은 그대로 표시한다.
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const id = new URLSearchParams(location.search).get('id');
const img = document.getElementById('img');

async function load() {
  const widget = await invoke('get_widget', { id });
  if (!widget) return;
  img.src = /^https?:\/\//i.test(widget.source) ? widget.source : convertFileSrc(widget.source);
}

listen('widget-updated', (event) => {
  if (event.payload === id) load();
});
load();
