import { __render } from '@/shared/helpter';

export interface CheckboxProps {
  checked: boolean;
  disabled?: boolean;
  onChange?: (checked: boolean) => void;
}

const props = withDefaults(defineProps<CheckboxProps>(), {
  disabled: false,
  onChange: () => {},
});

defineOptions({ name: 'Checkbox' });

export default __render<CheckboxProps>(() => (
  <input
    class="h-4 w-4 shrink-0 rounded border-outline-variant accent-primary focus:ring-primary"
    checked={props.checked}
    disabled={props.disabled}
    type="checkbox"
    onChange={(event) =>
      props.onChange((event.target as HTMLInputElement).checked)
    }
  />
));
