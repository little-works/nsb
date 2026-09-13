import { __render } from '@/shared/helper';

export interface RadioProps {
  checked: boolean;
  name?: string;
  value?: string;
  disabled?: boolean;
  onChange?: () => void;
}

const props = withDefaults(defineProps<RadioProps>(), {
  disabled: false,
  onChange: () => {},
});

defineOptions({ name: 'Radio' });

export default __render<RadioProps>(() => (
  <input
    class="h-4 w-4 shrink-0 border-outline-variant accent-primary focus:ring-primary"
    checked={props.checked}
    disabled={props.disabled}
    name={props.name}
    value={props.value}
    type="radio"
    onChange={props.onChange}
  />
));
