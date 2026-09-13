import { __render } from '@/shared/helper';
import {
  Popover as V0Popover,
  type PopoverActivatorSlotProps,
} from '@vuetify/v0';
import {
  cloneVNode,
  computed,
  onBeforeUnmount,
  ref,
  useSlots,
  type HTMLAttributes,
} from 'vue';
import type { Placement } from '@floating-ui/vue';

defineOptions({ name: 'Popover', inheritAttrs: false });

export type PopoverTrigger = 'click' | 'hover';

export interface PopoverProps {
  trigger?: PopoverTrigger;
  placement?: Placement;
  offset?: number;
  closeDelay?: number;
  referenceClass?: HTMLAttributes['class'];
  contentClass?: HTMLAttributes['class'];
  enterActiveClass?: string;
  enterFromClass?: string;
  enterToClass?: string;
  leaveActiveClass?: string;
  leaveFromClass?: string;
  leaveToClass?: string;
  disableTransition?: boolean;
}

export interface PopoverSlotProps {
  open: boolean;
  show: () => void;
  close: () => void;
  toggle: () => void;
}

const props = withDefaults(defineProps<PopoverProps>(), {
  trigger: 'hover',
  placement: 'bottom',
  offset: 4,
  closeDelay: 200,
});
const slots = useSlots();
const open = ref(false);
let closeTimer: ReturnType<typeof setTimeout> | undefined;

// CSS anchor positioning accepts physical area names for all Floating UI placements.
const positionArea = computed(() => {
  const placementMap: Record<Placement, string> = {
    top: 'top',
    'top-start': 'top left',
    'top-end': 'top right',
    right: 'right',
    'right-start': 'right top',
    'right-end': 'right bottom',
    bottom: 'bottom',
    'bottom-start': 'bottom left',
    'bottom-end': 'bottom right',
    left: 'left',
    'left-start': 'left top',
    'left-end': 'left bottom',
  };

  return placementMap[props.placement];
});

const contentOffsetStyle = computed(() => {
  const offset = `${props.offset}px`;

  if (props.placement.startsWith('top')) {
    return { margin: `0 0 ${offset}` };
  }
  if (props.placement.startsWith('right')) {
    return { margin: `0 0 0 ${offset}` };
  }
  if (props.placement.startsWith('left')) {
    return { margin: `0 ${offset} 0 0` };
  }

  return { margin: `${offset} 0 0` };
});

function clearCloseTimer() {
  if (closeTimer) {
    clearTimeout(closeTimer);
    closeTimer = undefined;
  }
}

function show() {
  clearCloseTimer();
  open.value = true;
}

function close() {
  clearCloseTimer();
  open.value = false;
}

function toggle() {
  if (open.value) {
    close();
  } else {
    show();
  }
}

function scheduleClose() {
  if (props.trigger !== 'hover') {
    return;
  }

  clearCloseTimer();
  closeTimer = setTimeout(close, props.closeDelay);
}

onBeforeUnmount(clearCloseTimer);

export default __render<PopoverProps>(() => (
  <V0Popover.Root
    modelValue={open.value}
    onUpdate:modelValue={(value) => {
      open.value = value;
    }}
    renderless
  >
    <V0Popover.Activator
      renderless
      v-slots={{
        default: ({ attrs }: PopoverActivatorSlotProps) => {
          const { popovertarget, ...activatorAttrs } = attrs;
          const activator = slots.default?.({
            close,
            open: open.value,
            show,
            toggle,
          });
          const vnode = activator?.[0];

          return vnode
            ? cloneVNode(vnode, {
                ...activatorAttrs,
                class: props.referenceClass,
                popovertarget:
                  props.trigger === 'click' ? popovertarget : undefined,
                onPointerenter: props.trigger === 'hover' ? show : undefined,
                onPointerleave:
                  props.trigger === 'hover' ? scheduleClose : undefined,
              })
            : null;
        },
      }}
    />
    <V0Popover.Content
      class={props.contentClass}
      positionArea={positionArea.value}
      positionTry="flip-block, flip-inline, flip-block flip-inline"
      style={contentOffsetStyle.value}
    >
      <div
        onPointerenter={props.trigger === 'hover' ? show : undefined}
        onPointerleave={props.trigger === 'hover' ? scheduleClose : undefined}
      >
        {slots.overlay?.({ close, open: open.value, show, toggle })}
      </div>
    </V0Popover.Content>
  </V0Popover.Root>
));
