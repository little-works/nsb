import { __render } from '@/shared/helper';
import type { HTMLAttributes } from 'vue';

export type DividerOrientation = 'horizontal' | 'vertical';

export interface DividerProps {
  orientation?: DividerOrientation;
}

const props = withDefaults(defineProps<DividerProps>(), {
  orientation: 'horizontal',
});

defineOptions({ name: 'Divider' });

export default __render<DividerProps & HTMLAttributes>(() => (
  <div
    aria-orientation={props.orientation}
    class={
      props.orientation === 'vertical'
        ? 'h-4 w-px shrink-0 bg-outline-variant'
        : 'h-px w-full bg-outline-variant'
    }
    role="separator"
  ></div>
));
