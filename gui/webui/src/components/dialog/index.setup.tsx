import { __render } from '@/shared/helpter';
import { IconButton } from '@/components/button';
import { CloseOutlined } from '@vicons/material';
import { Dialog as V0Dialog } from '@vuetify/v0';
import { useSlots, type HTMLAttributes } from 'vue';
import { useI18n } from 'vue-i18n';

export interface DialogProps {
  open: boolean;
  title?: string;
  description?: string;
  closeDisabled?: boolean;
  closeOnOverlayClick?: boolean;
  contentClass?: HTMLAttributes['class'];
  onClose?: () => void;
}

const props = withDefaults(defineProps<DialogProps>(), {
  title: '',
  description: '',
  closeDisabled: false,
  closeOnOverlayClick: true,
  contentClass: '',
});
const slots = useSlots();

function close() {
  if (!props.closeDisabled) {
    props.onClose?.();
  }
}

function handleOpenChange(open: boolean) {
  if (!open) {
    close();
  }
}

function preventCloseWhileDisabled(event: Event) {
  if (props.closeDisabled) {
    event.preventDefault();
  }
}

defineOptions({ name: 'Dialog' });
const { t } = useI18n();

export default __render<DialogProps>(() => {
  return (
    <V0Dialog.Root
      modelValue={props.open}
      onUpdate:modelValue={handleOpenChange}
    >
      {props.open ? (
        <V0Dialog.Content
          blocking={props.closeDisabled}
          class={[
            'm-auto w-full rounded p-5',
            'border border-outline-variant bg-surface shadow-2xl',
            props.contentClass,
          ]}
          closeOnClickOutside={props.closeOnOverlayClick}
          onCancel={preventCloseWhileDisabled}
          onClose={preventCloseWhileDisabled}
        >
          {props.title || props.description || props.onClose ? (
            <div class="mb-4 flex items-start justify-between gap-4">
              <div class="min-w-0">
                {props.title ? (
                  <V0Dialog.Title
                    as="h3"
                    class="text-lg font-semibold text-on-surface"
                  >
                    {props.title}
                  </V0Dialog.Title>
                ) : null}
                {props.description ? (
                  <V0Dialog.Description class="text-xs text-on-surface-variant">
                    {props.description}
                  </V0Dialog.Description>
                ) : null}
              </div>
              {props.onClose ? (
                <IconButton
                  aria-label={t('common.close')}
                  disabled={props.closeDisabled}
                  title={t('common.close')}
                  onClick={close}
                >
                  <CloseOutlined />
                </IconButton>
              ) : null}
            </div>
          ) : null}
          {slots.default?.()}
        </V0Dialog.Content>
      ) : null}
    </V0Dialog.Root>
  );
});
