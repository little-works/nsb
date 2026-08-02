import { readFileSync } from 'node:fs';
import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import vueJsx from '@vitejs/plugin-vue-jsx';
import vike from 'vike/plugin';
import { defineConfig } from 'vite';
import VueMacros from 'vue-macros/vite';

const apiOrigin = process.env.NSB_API_ORIGIN || 'http://127.0.0.1:8787';
const webuiPort = Number(process.env.NSB_WEBUI_PORT);
const guiCargoToml = readFileSync('../Cargo.toml', 'utf8');
const guiVersion = guiCargoToml.match(/^version\s*=\s*"([^"]+)"/mu)?.[1];

if (!guiVersion) {
  throw new Error('Unable to read the GUI version from ../Cargo.toml');
}

export default defineConfig({
  publicDir: '../assets',
  plugins: [
    vike(),
    tailwindcss(),
    VueMacros({
      setupSFC: true,
      setupComponent: false,
      defineRender: true,
      defineProps: true,
      plugins: {
        vue: vue({
          include: [/\.vue$/, /\.setup\.[cm]?[jt]sx?$/],
        }),
        vueJsx: vueJsx(),
      },
    }),
  ],
  base: '/webui/',
  define: {
    __NSB_VERSION__: JSON.stringify(guiVersion),
  },
  resolve: {
    alias: {
      '@': '/src',
    },
  },
  server: {
    host: true,
    port: webuiPort,
    proxy: {
      '/api': apiOrigin,
    },
  },
  css: {
    modules: {},
  },
});
