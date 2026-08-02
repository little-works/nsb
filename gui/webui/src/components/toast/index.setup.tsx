import { __render } from '@/shared/helpter';
import { IconButton } from '@/components/button';
import { Icon } from '@/components/icon';
import { useI18n } from 'vue-i18n';
import {
  CheckCircleOutlined,
  CloseOutlined,
  ErrorOutlined,
  WarningAmberOutlined,
} from '@vicons/material';

export type ToastVariant = 'info' | 'error' | 'warn';

export interface ToastOptions {
  title: string;
  content?: string;
  variant?: ToastVariant;
  closable?: boolean;
  duration?: number;
}

defineOptions({ name: 'Toast' });

interface ToastProps {
  title: string;
  content?: string;
  variant?: ToastVariant;
  closable?: boolean;
  duration?: number;
  onClose?: () => void;
  onMouseenter?: () => void;
  onMouseleave?: () => void;
}

const props = withDefaults(defineProps<ToastProps>(), {
  content: '',
  variant: 'info',
  closable: false,
  duration: 5000,
  onClose: () => {},
});

export default __render<ToastProps>(() => {
  const { t } = useI18n();
  const IconComponent =
    props.variant === 'error'
      ? ErrorOutlined
      : props.variant === 'warn'
        ? WarningAmberOutlined
        : CheckCircleOutlined;
  const iconClass =
    props.variant === 'error'
      ? 'text-error'
      : props.variant === 'warn'
        ? 'text-tertiary'
        : 'text-primary';
  const toastClass =
    props.variant === 'error'
      ? 'border-error'
      : props.variant === 'warn'
        ? 'border-tertiary'
        : 'border-primary';

  return (
    <div
      class={[
        'bg-surface-container-highest/50 backdrop-blur-sm',
        'flex min-w-64 max-w-sm gap-3 rounded border px-4 py-3 text-on-surface shadow-lg',
        props.content ? 'items-start' : 'items-center',
        toastClass,
      ]}
      onMouseenter={props.onMouseenter}
      onMouseleave={props.onMouseleave}
    >
      <Icon
        class={[props.content ? 'mt-1' : '', 'shrink-0 text-lg', iconClass]}
      >
        <IconComponent />
      </Icon>
      <div class="flex min-w-0 flex-1 select-text flex-col gap-1">
        <div class="text-sm font-semibold text-on-surface">{props.title}</div>
        {props.content ? (
          <div class="break-all text-xs text-on-surface-variant">
            {props.content}
          </div>
        ) : null}
      </div>
      {props.closable ? (
        <IconButton
          iconClass="text-on-surface-variant"
          title={t('common.closeNotification')}
          onClick={props.onClose}
        >
          <CloseOutlined />
        </IconButton>
      ) : null}
    </div>
  );
});
