import { __render } from '@/shared/helpter';
import { Icon } from '@/components/icon';
import { DonutLargeOutlined } from '@vicons/material';
import {
  useAttrs,
  useSlots,
  type ButtonHTMLAttributes,
  type HTMLAttributes,
} from 'vue';
import Button, { type ButtonProps } from './index.setup';

defineOptions({ name: 'IconButton', inheritAttrs: false });

export interface IconButtonProps {
  iconClass?: HTMLAttributes['class'];
}

const props = defineProps<IconButtonProps>();
const attrs = useAttrs();
const slots = useSlots();

export default __render<IconButtonProps & ButtonProps & ButtonHTMLAttributes>(
  () => (
    <Button
      {...attrs}
      iconOnly
      shape="square"
      size="sm"
      variant="ghost"
      v-slots={{
        icon: () => (
          <Icon class={['text-lg', props.iconClass]}>{slots.default?.()}</Icon>
        ),
        loading: () => (
          <Icon class={['animate-spin text-lg', props.iconClass]}>
            <DonutLargeOutlined />
          </Icon>
        ),
        tooltip: slots.tooltip,
      }}
    />
  ),
);
