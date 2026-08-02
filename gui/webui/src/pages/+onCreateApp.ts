import { type PageContext } from 'vike/types';
import { VueQueryPlugin } from '@tanstack/vue-query';
import { createTooltipPlugin } from '@vuetify/v0';
import { i18n } from '@/i18n';

export function onCreateApp(pageContext: PageContext) {
  if (pageContext.isRenderingHead) {
    // Don't add plugins when rendering <head> (see Lifecycle)
    return;
  }
  const app = pageContext.app;
  app?.use(i18n);
  app?.use(VueQueryPlugin);
  app?.use(
    createTooltipPlugin({
      openDelay: 700,
      closeDelay: 150,
      skipDelay: 300,
    }),
  );
}
