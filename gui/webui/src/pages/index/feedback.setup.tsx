import { __render } from '@/shared/helpter';
import { Icon } from '@/components/icon';
import { Tooltip } from '@/components/tooltip';
import { WarningAmberOutlined } from '@vicons/material';
import { useI18n } from 'vue-i18n';

export interface OverviewFeedbackProps {
  errorMessage: string;
  loading: boolean;
  kernelInstalled: boolean;
  kernelRunning: boolean;
  hasGroups: boolean;
}

const props = defineProps<OverviewFeedbackProps>();

defineOptions({ name: 'OverviewFeedback' });
const { t } = useI18n();

export default __render<OverviewFeedbackProps>(() => {
  return (
    <>
      {props.errorMessage ? (
        <div class="rounded border border-danger/35 bg-danger/8 px-4 py-3 text-sm leading-5 text-danger">
          {props.errorMessage}
        </div>
      ) : null}

      {!props.loading && !props.kernelRunning ? (
        <div class="flex flex-wrap items-center gap-2 rounded border border-outline-variant bg-surface-container-low p-4 text-sm text-on-surface-variant">
          <span>{t('home.kernelStopped')}</span>
          {!props.kernelInstalled ? (
            <Tooltip content={t('traffic.noCoreAction')}>
              <a
                aria-label={t('traffic.noCoreAction')}
                href="/webui/settings"
                class="flex h-6 items-center gap-1 rounded-full bg-surface-container-high px-2
                  text-xs text-tertiary outline-none
                  transition-colors hover:bg-surface-container-highest
                  focus-visible:ring-2 focus-visible:ring-tertiary"
              >
                <Icon class="text-sm">
                  <WarningAmberOutlined />
                </Icon>
                <span>{t('traffic.noCore')}</span>
              </a>
            </Tooltip>
          ) : null}
        </div>
      ) : null}

      {!props.loading &&
      props.kernelRunning &&
      !props.hasGroups &&
      !props.errorMessage ? (
        <div class="rounded border border-outline-variant bg-surface-container-low p-4 text-sm text-on-surface-variant">
          {t('home.noGroups')}
        </div>
      ) : null}
    </>
  );
});
