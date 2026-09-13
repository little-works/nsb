import {
  Popover,
  type PopoverProps,
  type PopoverSlotProps,
} from '@/components/popover';
import { __render } from '@/shared/helper';
import { useAttrs, useSlots, type HTMLAttributes } from 'vue';

defineOptions({ name: 'Dropdown', inheritAttrs: false });

export interface DropdownProps {
  contentClass?: HTMLAttributes['class'];
}

export type DropdownSlotProps = PopoverSlotProps;

const props = defineProps<DropdownProps>();
const attrs = useAttrs();
const slots = useSlots();

export default __render<DropdownProps & PopoverProps>(() => (
  <Popover
    placement="bottom-start"
    {...attrs}
    referenceClass="inline-flex"
    contentClass={[
      'z-dropdown min-w-32 overflow-hidden rounded px-2 py-2',
      'border border-outline-variant bg-surface-container-highest/10 shadow-lg backdrop-blur-sm',
      props.contentClass,
    ]}
    enterActiveClass="transition-[opacity,transform] duration-150 ease-out"
    enterFromClass="scale-95 opacity-0"
    enterToClass="scale-100 opacity-100"
    leaveActiveClass="transition-[opacity,transform] duration-100 ease-in"
    leaveFromClass="scale-100 opacity-100"
    leaveToClass="scale-95 opacity-0"
    v-slots={{
      default: (slotProps: DropdownSlotProps) => slots.default?.(slotProps),
      overlay: (slotProps: DropdownSlotProps) => slots.overlay?.(slotProps),
    }}
  />
));
