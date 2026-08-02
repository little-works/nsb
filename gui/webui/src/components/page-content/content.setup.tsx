import { __render } from '@/shared/helpter';
import type { HTMLAttributes } from 'vue';
import { useSlots } from 'vue';

export interface PageContentProps {}

const slots = useSlots();

defineOptions({ name: 'PageContent' });

export default __render<PageContentProps & HTMLAttributes>(() => {
  return <div class="mx-auto max-w-6xl px-4">{slots.default?.()}</div>;
});
