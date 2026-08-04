import { __render } from '@/shared/helpter';
import { useI18n } from 'vue-i18n';

export interface OverviewFeedbackProps {
  errorMessage: string;
  loading: boolean;
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
        <div class="rounded border border-outline-variant bg-surface-container-low p-4 text-sm text-on-surface-variant">
          {t('home.kernelStopped')}
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
