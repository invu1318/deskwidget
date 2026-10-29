import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

const page = (name) => fileURLToPath(new URL(`./${name}.html`, import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  // Rust 빌드 폴더는 감시하지 않는다. 빌드 중 잠긴 파일 때문에 개발 서버가 멈춘다.
  server: { port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**'] } },
  build: {
    rollupOptions: {
      input: {
        settings: page('settings'),
        widget: page('widget'),
        overlay: page('overlay'),
      },
    },
  },
});
