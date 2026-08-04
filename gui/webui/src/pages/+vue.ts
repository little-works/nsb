import type { Config } from 'vike/types';

// https://vike.dev/vue-setting
export default {
  keepAlive: {
    exclude: 'ProfileEditPage',
  },
} satisfies Config['vue'];
