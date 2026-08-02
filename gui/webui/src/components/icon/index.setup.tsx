import { __render } from '@/shared/helpter';
import { computed, useSlots, type HTMLAttributes } from 'vue';
import './style.css';

export interface IconProps {
  size?: string | number;
  color?: string;
  tag?: string;
}

const props = defineProps<IconProps>();
const slots = useSlots();
const style = computed(() => {
  const size =
    typeof props.size === 'number' || /^\d+$/.test(String(props.size ?? ''))
      ? `${props.size}px`
      : props.size;
  return {
    fontSize: size,
    color: props.color,
  };
});

defineOptions({ name: 'Icon' });

export default __render<IconProps & HTMLAttributes>(() => {
  const Tag = props.tag || 'span';

  return (
    <Tag
      // @ts-expect-error
      class={
        'nsb-icon inline-flex shrink-0 items-center justify-center leading-none'
      }
      style={style.value}
    >
      {slots.default?.()}
    </Tag>
  );
});
