// 앱이 직접 그리는 위젯(사진·동영상, 슬라이드쇼, 시계, 메모) 페이지.
// 로컬 파일은 asset 프로토콜로, URL은 그대로 표시한다. mp4/webm은 소리 없이 반복 재생한다.
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const id = new URLSearchParams(location.search).get('id');
const $ = (name) => document.getElementById(name);
const img = $('img');
const video = $('video');
const still = $('still');
const clock = $('clock');
const memo = $('memo');
const VIDEO_EXT = /\.(mp4|webm)$/i;
const toSrc = (source) => (/^https?:\/\//i.test(source) ? source : convertFileSrc(source));

let loaded = '';
let paused = false;
let slides = [];
let slideIndex = 0;
let slideTimer = 0;
let clockTimer = 0;

function show(element) {
  for (const each of [img, video, still, clock, memo]) each.hidden = each !== element;
}

function tick() {
  const now = new Date();
  $('time').textContent = [now.getHours(), now.getMinutes(), now.getSeconds()]
    .map((n) => String(n).padStart(2, '0'))
    .join(':');
  $('date').textContent = now.toLocaleDateString('ko-KR', { month: 'long', day: 'numeric', weekday: 'long' });
}

function nextSlide() {
  if (paused || slides.length < 2) return;
  slideIndex = (slideIndex + 1) % slides.length;
  img.src = convertFileSrc(slides[slideIndex]);
}

async function load() {
  const widget = await invoke('get_widget', { id });
  if (!widget) return;
  // 불투명도 같은 설정만 바뀌면 재생 중인 내용을 다시 불러오지 않는다. 메모 내용은 이 페이지가 주인이다.
  const key = JSON.stringify([widget.kind, widget.kind === 'memo' ? '' : widget.source, widget.slideSeconds]);
  if (key === loaded) return;
  loaded = key;
  clearInterval(slideTimer);
  clearInterval(clockTimer);

  const isVideo = widget.kind === 'image' && VIDEO_EXT.test(widget.source.split(/[?#]/)[0]);
  if (!isVideo) {
    video.removeAttribute('src');
    video.load();
    invoke('set_media_ratio', { id, ratio: null });
  }
  if (widget.kind === 'clock') {
    show(clock);
    tick();
    clockTimer = setInterval(tick, 1000);
  } else if (widget.kind === 'memo') {
    show(memo);
    memo.value = widget.source;
  } else if (widget.kind === 'slideshow') {
    slides = await invoke('list_slides', { id });
    slideIndex = 0;
    show(img);
    if (slides.length) img.src = convertFileSrc(slides[0]);
    else img.removeAttribute('src');
    slideTimer = setInterval(nextSlide, widget.slideSeconds * 1000);
  } else if (isVideo) {
    show(video);
    video.src = toSrc(widget.source);
    if (!paused) video.play().catch(() => {});
  } else {
    show(img);
    img.src = toSrc(widget.source);
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

// 전체화면 앱이 떠 있는 동안 움직임을 멈춘다. 움직이는 사진은 현재 장면을 그림으로 바꿔 둔다.
function setPaused(on) {
  paused = on;
  if (!video.hidden) {
    if (on) video.pause();
    else video.play().catch(() => {});
  } else if (on && !img.hidden && img.naturalWidth) {
    still.width = img.naturalWidth;
    still.height = img.naturalHeight;
    still.getContext('2d').drawImage(img, 0, 0);
    show(still);
  } else if (!on && !still.hidden) {
    show(img);
  }
}
listen('playback-paused', (event) => setPaused(event.payload));

let memoTimer = 0;
memo.addEventListener('input', () => {
  clearTimeout(memoTimer);
  memoTimer = setTimeout(() => invoke('set_memo_text', { id, text: memo.value }), 400);
});

listen('widget-updated', (event) => {
  if (event.payload === id) load();
});
load();
