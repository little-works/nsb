import { type PageContext } from 'vike/types';
import { VueQueryPlugin } from '@tanstack/vue-query';
import { createTooltipPlugin } from '@vuetify/v0';
import { i18n } from '@/i18n';

export function onCreateApp(pageContext: PageContext) {
  const { app } = pageContext;
  if (!app) {
    throw new Error('Vike did not create a Vue app');
  }

  app.use(i18n);

  if (pageContext.isRenderingHead) {
    // The head renderer needs i18n, but not the application-only plugins below.
    return;
  }
  app.use(VueQueryPlugin);
  app.use(
    createTooltipPlugin({
      openDelay: 700,
      closeDelay: 150,
      skipDelay: 300,
    }),
  );
}
