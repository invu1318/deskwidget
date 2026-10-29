// 움직이는 사진·동영상 위젯: 로컬 파일은 asset 프로토콜로, URL은 그대로 표시한다.
// mp4/webm은 소리 없이 반복 재생한다.
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const id = new URLSearchParams(location.search).get('id');
const img = document.getElementById('img');
const video = document.getElementById('video');
const VIDEO_EXT = /\.(mp4|webm)$/i;

async function load() {
  const widget = await invoke('get_widget', { id });
  if (!widget) return;
  const src = /^https?:\/\//i.test(widget.source) ? widget.source : convertFileSrc(widget.source);
  const isVideo = VIDEO_EXT.test(widget.source.split(/[?#]/)[0]);
  img.hidden = isVideo;
  video.hidden = !isVideo;
  if (isVideo) {
    img.removeAttribute('src');
    video.src = src;
    video.play().catch(() => {});
  } else {
    video.removeAttribute('src');
    video.load();
    img.src = src;
    invoke('set_media_ratio', { id, ratio: null });
  }
}

// 동영상 요소를 창 안에 들어가는 영상 비율 그대로의 크기로 맞춰, 검은 여백이 생기지 않게 한다.
function fitVideo() {
  if (video.hidden || !video.videoWidth || !video.videoHeight) return;
  const scale = Math.min(innerWidth / video.videoWidth, innerHeight / video.videoHeight);
  video.style.width = `${Math.floor(video.videoWidth * scale)}px`;
  video.style.height = `${Math.floor(video.videoHeight * scale)}px`;
}
video.addEventListener('loadedmetadata', () => {
  fitVideo();
  // 앱이 위젯 창을 영상 비율에 맞춰 여백(검게 칠해짐)을 없앤다.
  invoke('set_media_ratio', { id, ratio: video.videoWidth / video.videoHeight });
});
addEventListener('resize', fitVideo);

document.addEventListener('contextmenu', (event) => {
  event.preventDefault();
  invoke('show_widget_menu', { id });
});

listen('widget-updated', (event) => {
  if (event.payload === id) load();
});
load();
