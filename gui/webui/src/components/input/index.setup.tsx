import { __render } from '@/shared/helpter';
import { computed, useAttrs, type InputHTMLAttributes } from 'vue';

defineOptions({ name: 'Input', inheritAttrs: false });

export interface InputProps {
  block?: boolean;
  size?: 'sm' | 'md';
}

const props = withDefaults(defineProps<InputProps>(), {
  block: true,
  size: 'md',
});

const attrs = useAttrs();

const className = computed(() => [
  props.block ? 'w-full' : 'inline-block',
  props.size === 'sm' ? 'h-8 px-2.5 text-xs' : 'h-9 px-3 text-sm',
  'rounded border',
  'border-outline-variant bg-surface text-on-surface',
  'outline-none placeholder:text-on-surface-variant',
  'transition-[border-color,box-shadow] focus:border-primary focus-visible:outline-2 focus-visible:outline-primary focus-visible:outline-offset-2',
  'disabled:cursor-not-allowed disabled:opacity-60',
]);

export default __render<InputProps & InputHTMLAttributes>(() => {
  const { class: customClass, ...inputAttrs } = attrs;

  return <input {...inputAttrs} class={[className.value, customClass]} />;
});
