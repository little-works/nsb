import vikeVue from 'vike-vue/config';
import vikeVuePinia from 'vike-vue-pinia/config';
import type { Config } from 'vike/types';

export default {
  extends: [vikeVue, vikeVuePinia],
  title: 'NSB',
  description: 'NSB Desktop WebUI',
  ssr: false,
  // Generate a shell per route; page content is rendered only in the browser.
  prerender: true,
  htmlAttributes: {
    'data-nsb-theme': '$data_nsb_theme',
  },
} satisfies Config;
