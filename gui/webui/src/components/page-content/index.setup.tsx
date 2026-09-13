import { __render } from '@/shared/helper';
import { useSlots, type HTMLAttributes } from 'vue';
import PageHeader from './header.setup';

export interface PageProps {
  title?: string;
  subtitle?: string;
}

const props = withDefaults(defineProps<PageProps>(), {
  title: '',
  subtitle: '',
});
const slots = useSlots();

defineOptions({ name: 'Page' });

export default __render<PageProps & HTMLAttributes>(() => (
  <div class="min-h-screen bg-background">
    {slots.header?.() ?? (
      <PageHeader title={props.title} subtitle={props.subtitle}>
        {{
          actions: slots.actions,
        }}
      </PageHeader>
    )}
    <main class="h-[calc(100vh-3.5rem)] overflow-y-auto">
      {slots.default?.()}
    </main>
    {slots.overlay?.()}
  </div>
));
