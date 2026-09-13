import { __render } from '@/shared/helper';
import { IconButton } from '@/components/button';
import { useFloatingDockStore } from '@/store/floating-dock';
import { VerticalAlignTopOutlined } from '@vicons/material';
import { useI18n } from 'vue-i18n';
import './style.css';

const floatingDockStore = useFloatingDockStore();

function scrollToTop() {
  document.querySelector('main')?.scrollTo({ behavior: 'smooth', top: 0 });
}

defineOptions({ name: 'FloatingDock' });
const { t } = useI18n();

export default __render(() => {
  const content = floatingDockStore.content;

  return (
    <div class="floating-dock fixed bottom-12 right-6 z-floating-dock flex max-h-76 flex-col items-center rounded border border-outline-variant bg-surface-container-lowest p-2 shadow-sm">
      <div class="floating-dock__items flex flex-col items-center">
        <IconButton title={t('common.goToTop')} onClick={scrollToTop}>
          <VerticalAlignTopOutlined />
        </IconButton>
        {content ? (
          <div class="my-1 h-px w-6 shrink-0 bg-outline-variant"></div>
        ) : null}
        {content?.()}
      </div>
    </div>
  );
});
