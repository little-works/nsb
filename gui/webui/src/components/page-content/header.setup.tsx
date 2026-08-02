import { __render } from '@/shared/helpter';
import { useSlots } from 'vue';

export interface PageHeaderProps {
  title?: string;
  subtitle?: string;
}

const props = withDefaults(defineProps<PageHeaderProps>(), {
  title: '',
  subtitle: '',
});
const slots = useSlots();

defineOptions({ name: 'PageHeader' });

export default __render<PageHeaderProps>(() => {
  return (
    <header class="sticky top-0 z-page-header flex h-14 items-center justify-between border-b border-outline-variant bg-surface px-4">
      <div class="min-w-0">
        <h2 class="truncate text-lg font-semibold leading-6 text-on-surface">
          {props.title}
        </h2>
        {props.subtitle ? (
          <p class="truncate text-xs text-on-surface-variant">
            {props.subtitle}
          </p>
        ) : null}
      </div>
      {slots.default?.()}
      {slots.actions ? (
        <div class="flex shrink-0 items-center gap-2">{slots.actions()}</div>
      ) : null}
    </header>
  );
});
