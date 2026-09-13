import { __render } from '@/shared/helpter';
import { Icon } from '@/components/icon';
import { Tooltip, type TooltipWithPopoverProps } from '@/components/tooltip';
import { DonutLargeOutlined } from '@vicons/material';
import {
  computed,
  onBeforeUnmount,
  ref,
  useAttrs,
  useSlots,
  type ButtonHTMLAttributes,
} from 'vue';

defineOptions({ name: 'Button', inheritAttrs: false });

export type ButtonVariant =
  'solid' | 'outline' | 'subtle' | 'ghost' | 'primary-ghost' | 'danger-ghost';
export type ButtonShape = 'pill' | 'rect' | 'square';
export type ButtonSize = 'xs' | 'sm' | 'field' | 'md';

export interface ButtonProps {
  variant?: ButtonVariant;
  shape?: ButtonShape;
  size?: ButtonSize;
  tag?: string;
  nativeType?: 'button' | 'submit' | 'reset';
  disabled?: boolean;
  block?: boolean;
  iconOnly?: boolean;
  tooltip?: string;
  tooltipProps?: TooltipWithPopoverProps;
  onClick?: (event: MouseEvent) => void | Promise<void>;
}

const props = withDefaults(defineProps<ButtonProps>(), {
  variant: 'outline',
  shape: 'rect',
  size: 'sm',
  tag: 'button',
  nativeType: 'button',
  disabled: false,
  block: false,
  iconOnly: false,
});

const slots = useSlots();
const attrs = useAttrs();
const hasTooltip = computed(() =>
  Boolean(props.tooltipProps?.content || props.tooltip || slots.tooltip),
);
const pending = ref(false);
const loading = ref(false);
let loadingTimer: ReturnType<typeof setTimeout> | undefined;

const sizeClassMap: Record<ButtonShape, Record<ButtonSize, string>> = {
  pill: {
    xs: 'h-7 rounded-full px-2.5 text-xs',
    sm: 'h-8 rounded-full px-3 text-xs',
    field: 'h-9 rounded-full px-3 text-sm',
    md: 'h-10 rounded-full px-4 text-sm',
  },
  rect: {
    xs: 'h-7 rounded px-2.5 text-xs',
    sm: 'h-8 rounded px-3 text-xs',
    field: 'h-9 rounded px-3 text-sm',
    md: 'h-10 rounded px-4 text-sm',
  },
  square: {
    xs: 'h-7 w-7 rounded text-xs',
    sm: 'h-8 w-8 rounded text-xs',
    field: 'h-9 w-9 rounded text-sm',
    md: 'h-10 w-10 rounded text-sm',
  },
};

const variantClassMap: Record<ButtonVariant, string> = {
  solid:
    'bg-primary text-on-primary hover:bg-primary-container active:bg-primary disabled:hover:bg-primary disabled:active:bg-primary',
  outline:
    'border border-outline-variant bg-transparent text-on-surface hover:border-primary hover:text-primary active:border-primary active:bg-surface-container-low active:text-primary',
  subtle:
    'bg-surface-container-highest text-on-surface hover:bg-outline-variant active:bg-outline',
  ghost:
    'text-on-surface-variant hover:bg-surface-container hover:text-primary active:bg-surface-container-high active:text-primary',
  'primary-ghost':
    'text-primary hover:bg-primary-container hover:text-primary-fg active:bg-primary-container active:text-primary-fg',
  'danger-ghost':
    'text-error hover:bg-error-container active:bg-error-container',
};

const className = computed(() => [
  props.block ? 'flex w-full' : 'inline-flex shrink-0',
  'items-center justify-center gap-1.5 whitespace-nowrap font-medium leading-none transition-[color,background-color,border-color,transform] active:translate-y-px disabled:cursor-not-allowed disabled:opacity-50 disabled:active:translate-y-0',
  sizeClassMap[props.shape][props.size],
  variantClassMap[props.variant],
]);

function clearLoadingTimer() {
  if (loadingTimer) {
    clearTimeout(loadingTimer);
    loadingTimer = undefined;
  }
}

async function handleClick(event: MouseEvent) {
  if (props.disabled || pending.value) {
    event.preventDefault();
    return;
  }

  if (!props.onClick) {
    return;
  }

  pending.value = true;
  loadingTimer = setTimeout(() => {
    loading.value = true;
  }, 200);

  try {
    await props.onClick(event);
  } finally {
    clearLoadingTimer();
    loading.value = false;
    pending.value = false;
  }
}

onBeforeUnmount(clearLoadingTimer);

function renderLoadingSlot() {
  return (
    slots.loading?.() ?? (
      <Icon class="animate-spin text-base">
        <DonutLargeOutlined />
      </Icon>
    )
  );
}

function renderIconSlot() {
  return loading.value ? renderLoadingSlot() : slots.icon?.();
}

export default __render<ButtonProps & ButtonHTMLAttributes>(() => {
  const Tag = props.tag;
  const disabled = props.disabled || pending.value;
  const buttonType =
    Tag === 'button' ? { type: props.nativeType, disabled } : {};
  const disabledState =
    Tag === 'button' ? {} : { 'aria-disabled': disabled || undefined };

  const renderButton = () => (
    <Tag
      {...attrs}
      {...buttonType}
      {...disabledState}
      // @ts-ignore
      class={className.value}
      onClick={handleClick}
    >
      {props.iconOnly ? (
        loading.value ? (
          renderLoadingSlot()
        ) : (
          (slots.icon?.() ?? slots.default?.())
        )
      ) : (
        <>
          {loading.value || slots.icon ? renderIconSlot() : null}
          {slots.default?.()}
        </>
      )}
    </Tag>
  );

  if (!hasTooltip.value) {
    return renderButton();
  }

  return (
    <Tooltip
      {...props.tooltipProps}
      content={props.tooltipProps?.content || props.tooltip}
      v-slots={{
        default: renderButton,
        overlay: slots.tooltip,
      }}
    />
  );
});
