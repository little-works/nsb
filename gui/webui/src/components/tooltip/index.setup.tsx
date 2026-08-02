import { __render } from '@/shared/helpter';
import {
  Tooltip as V0Tooltip,
  type TooltipActivatorSlotProps,
} from '@vuetify/v0';
import { cloneVNode, computed, useSlots, type HTMLAttributes } from 'vue';

defineOptions({ name: 'Tooltip', inheritAttrs: false });

export interface TooltipProps {
  content?: string;
  contentClass?: HTMLAttributes['class'];
  openDelay?: number;
  closeDelay?: number;
  disabled?: boolean;
}

export type TooltipWithPopoverProps = TooltipProps;

const props = withDefaults(defineProps<TooltipProps>(), {
  content: '',
});
const slots = useSlots();
const hasContent = computed(() => Boolean(props.content || slots.overlay));

export default __render<TooltipWithPopoverProps>(() => {
  if (!hasContent.value) {
    return slots.default?.();
  }

  return (
    <V0Tooltip.Root
      closeDelay={props.closeDelay}
      disabled={props.disabled}
      openDelay={props.openDelay}
      positionArea="top"
      positionTry="flip-block"
      renderless
    >
      <V0Tooltip.Activator
        renderless
        v-slots={{
          default: ({ attrs, styles }: TooltipActivatorSlotProps) => {
            const activator = slots.default?.()[0];
            return activator
              ? cloneVNode(activator, { ...attrs, style: styles })
              : null;
          },
        }}
      />
      <V0Tooltip.Content
        class={[
          'z-dropdown my-2 max-w-64 rounded-xs px-3 py-2',
          'bg-inverse-surface text-center text-xs text-surface shadow-lg',
          props.contentClass,
        ]}
      >
        {slots.overlay?.() ?? props.content}
      </V0Tooltip.Content>
    </V0Tooltip.Root>
  );
});
