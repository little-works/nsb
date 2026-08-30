import { __render } from '@/shared/helpter';

export interface CheckboxProps {
  checked: boolean;
  indeterminate?: boolean;
  disabled?: boolean;
  onChange?: (checked: boolean) => void;
}

const props = withDefaults(defineProps<CheckboxProps>(), {
  disabled: false,
  indeterminate: false,
  onChange: () => {},
});

defineOptions({ name: 'Checkbox' });

export default __render<CheckboxProps>(() => (
  <input
    class="h-4 w-4 shrink-0 rounded border-outline-variant accent-primary focus:ring-primary"
    checked={props.checked}
    disabled={props.disabled}
    indeterminate={props.indeterminate}
    aria-checked={props.indeterminate ? 'mixed' : props.checked}
    type="checkbox"
    onChange={(event) =>
      props.onChange((event.target as HTMLInputElement).checked)
    }
  />
));
