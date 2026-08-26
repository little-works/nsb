import { __render } from '@/shared/helpter';
import {
  Select as V0Select,
  type SelectCueSlotProps,
  type SelectItemSlotProps,
} from '@vuetify/v0';
import { computed } from 'vue';

defineOptions({ name: 'Select' });

export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectProps {
  modelValue: string;
  options: readonly SelectOption[];
  disabled?: boolean;
  block?: boolean;
  size?: 'sm' | 'md';
  ariaLabel: string;
  onUpdateModelValue?: (value: string) => void;
}

const props = withDefaults(defineProps<SelectProps>(), {
  disabled: false,
  block: false,
  size: 'md',
});

const selectedOption = computed(() =>
  props.options.find((option) => option.value === props.modelValue),
);

function handleModelValueUpdate(value: string | string[] | undefined) {
  if (typeof value === 'string') {
    props.onUpdateModelValue?.(value);
  }
}

export default __render<SelectProps>(() => (
  <V0Select.Root
    disabled={props.disabled}
    mandatory
    modelValue={props.modelValue}
    onUpdate:modelValue={handleModelValueUpdate}
  >
    <V0Select.Activator
      aria-label={props.ariaLabel}
      class={[
        'flex items-center justify-between gap-3 rounded border whitespace-nowrap',
        props.block ? 'w-full' : 'min-w-32',
        props.size === 'sm' ? 'h-8 px-2.5 text-xs' : 'h-9 px-3 text-sm',
        'border-outline-variant bg-surface text-on-surface',
        'cursor-pointer focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-2',
        'disabled:cursor-not-allowed disabled:opacity-60',
      ]}
    >
      <V0Select.Value
        class="min-w-0 flex-1 truncate text-left"
        v-slots={{
          default: () => selectedOption.value?.label,
        }}
      />
      <V0Select.Cue
        class="shrink-0 text-xs text-on-surface-variant"
        v-slots={{
          default: ({ isOpen }: SelectCueSlotProps) => (isOpen ? '▴' : '▾'),
        }}
      />
    </V0Select.Activator>
    <V0Select.Content
      class={[
        'z-sidebar mt-1 overflow-hidden rounded border p-1 shadow-lg',
        props.size === 'sm' ? 'text-xs' : 'text-sm',
        'border-outline-variant bg-surface text-on-surface',
      ]}
      style={{ minWidth: 'anchor-size(width)' }}
    >
      {props.options.map((option) => (
        <V0Select.Item
          key={option.value}
          id={option.value}
          value={option.value}
          v-slots={{
            default: ({ isHighlighted, isSelected }: SelectItemSlotProps) => (
              <div
                class={[
                  'cursor-default select-none rounded',
                  props.size === 'sm' ? 'px-2.5 py-1.5' : 'px-3 py-2',
                  isHighlighted
                    ? 'bg-primary text-on-primary'
                    : isSelected
                      ? 'font-medium text-primary'
                      : 'hover:bg-surface-container-high',
                ]}
              >
                {option.label}
              </div>
            ),
          }}
        />
      ))}
    </V0Select.Content>
  </V0Select.Root>
));
