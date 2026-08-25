import { __render } from '@/shared/helpter';
import {
  Select as V0Select,
  type SelectCueSlotProps,
  type SelectItemSlotProps,
  type SelectRootSlotProps,
  useClickOutside,
} from '@vuetify/v0';
import { computed, ref } from 'vue';

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
  ariaLabel: string;
  onUpdateModelValue?: (value: string) => void;
}

const props = withDefaults(defineProps<SelectProps>(), {
  disabled: false,
  block: false,
});

const selectedOption = computed(() =>
  props.options.find((option) => option.value === props.modelValue),
);
const selectRoot = ref<HTMLElement | null>(null);
const closeSelect = ref<(() => void) | null>(null);

useClickOutside(selectRoot, () => {
  closeSelect.value?.();
});

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
    v-slots={{
      default: ({ id, isOpen, close }: SelectRootSlotProps) => {
        closeSelect.value = close;

        return (
          <div ref={selectRoot} class="relative">
            <V0Select.Activator
              aria-label={props.ariaLabel}
              class={[
                'flex h-9 items-center justify-between gap-3 rounded border px-3 whitespace-nowrap',
                props.block ? 'w-full' : 'min-w-32',
                'border-outline-variant bg-surface text-sm text-on-surface',
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
                  default: ({ isOpen }: SelectCueSlotProps) =>
                    isOpen ? '▴' : '▾',
                }}
              />
            </V0Select.Activator>
            {isOpen ? (
              <div
                id={`${id}-listbox`}
                role="listbox"
                aria-label={props.ariaLabel}
                class={[
                  'absolute left-0 top-full z-sidebar mt-1 min-w-full overflow-hidden rounded border p-1 shadow-lg',
                  'border-outline-variant bg-surface text-sm text-on-surface',
                ]}
              >
                {props.options.map((option) => (
                  <V0Select.Item
                    key={option.value}
                    id={option.value}
                    value={option.value}
                    v-slots={{
                      default: ({
                        isHighlighted,
                        isSelected,
                      }: SelectItemSlotProps) => (
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
              </div>
            ) : null}
          </div>
        );
      },
    }}
  />
));
