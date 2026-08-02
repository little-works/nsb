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
  ariaLabel: string;
  onUpdateModelValue?: (value: string) => void;
}

const props = withDefaults(defineProps<SelectProps>(), {
  disabled: false,
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
        'flex h-9 min-w-32 items-center justify-between gap-3 rounded border px-3',
        'border-outline-variant bg-surface text-sm text-on-surface',
        'cursor-pointer focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-2',
        'disabled:cursor-not-allowed disabled:opacity-60',
      ]}
    >
      <V0Select.Value
        v-slots={{
          default: () => selectedOption.value?.label,
        }}
      />
      <V0Select.Cue
        class="text-xs text-on-surface-variant"
        v-slots={{
          default: ({ isOpen }: SelectCueSlotProps) => (isOpen ? '▴' : '▾'),
        }}
      />
    </V0Select.Activator>
    <V0Select.Content
      class={[
        'z-dropdown mt-1 overflow-hidden rounded border p-1 shadow-lg',
        'border-outline-variant bg-surface text-sm text-on-surface',
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
                  'cursor-default select-none rounded px-3 py-2',
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
