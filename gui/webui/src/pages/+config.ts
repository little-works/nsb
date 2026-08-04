import vikeVue from 'vike-vue/config';
import vikeVuePinia from 'vike-vue-pinia/config';
import type { Config } from 'vike/types';

const isDev = process.env.NODE_ENV === 'development';

export default {
  extends: [vikeVue, vikeVuePinia],
  title: 'NSB',
  description: 'NSB Desktop WebUI',
  ssr: true,
  prerender: true,
  htmlAttributes: {
    'data-nsb-theme': '$data_nsb_theme',
  },
} satisfies Config;
