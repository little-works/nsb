import { __render } from '@/shared/helpter';
import {
  ArrowDownwardOutlined,
  ArrowUpwardOutlined,
  CheckCircleFilled,
  ErrorOutlined,
  RadioButtonUncheckedFilled,
  RestartAltOutlined,
  SpeedOutlined,
  WarningAmberOutlined,
} from '@vicons/material';
import { Tooltip } from '@/components/tooltip';
import { Button } from '@/components/button';
import { toast } from '@/components/toast';
import { restartKernel } from '@/api/client';
import { useClientQuery } from '@/hooks/use-client-query';
import { runtimeQueryKey, useRuntimeStatus } from '@/store/app';
import { useScoreStreamData } from '@/hooks/use-score-stream';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Icon } from '@/components/icon';
import { useQueryClient } from '@tanstack/vue-query';
import { i18n } from '@/i18n';

const { traffic, connectionState, failureNotificationVersion } =
  useScoreStreamData();
const runtimeStatus = useRuntimeStatus();
const queryClient = useQueryClient();
const { t } = useI18n();
const coreStatus = computed(() => {
  const kernel = runtimeStatus.data.value?.kernel;
  const installed = kernel?.installed ?? true;
  if (!installed) {
    return {
      icon: WarningAmberOutlined,
      label: t('traffic.noCore'),
      class: 'text-tertiary',
      href: '/webui/settings',
      tooltip: t('traffic.noCoreAction'),
    };
  }
  if (kernel?.status === 'Running') {
    return {
      icon: CheckCircleFilled,
      label: t('traffic.running'),
      class: 'text-secondary',
      href: undefined,
      tooltip: undefined,
    };
  }
  if (kernel?.status === 'Failed') {
    return {
      icon: ErrorOutlined,
      label: t('traffic.failed'),
      class: 'text-error',
      href: '/webui/logs',
      tooltip: t('traffic.failedAction'),
    };
  }
  return {
    icon: RadioButtonUncheckedFilled,
    label: t('traffic.stopped'),
    class: 'text-on-surface-variant',
    href: undefined,
    tooltip: undefined,
  };
});

let actionStartedFromFailed = false;
let failureNotificationVersionAtStart = 0;
const restartQuery = useClientQuery({
  queryKey: ['kernel', 'restart'],
  enabled: false,
  retry: false,
  queryFn: async () => {
    try {
      const snapshot = await restartKernel();
      queryClient.setQueryData(runtimeQueryKey, snapshot);
      return snapshot;
    } catch (error) {
      if (
        connectionState.value !== 'connected' ||
        actionStartedFromFailed ||
        failureNotificationVersion.value === failureNotificationVersionAtStart
      ) {
        toast.error({
          title: i18n.global.t('errors.kernelStartFailed'),
          content: i18n.global.t('errors.kernelStartFailedAction'),
        });
      }
      throw error;
    } finally {
      await runtimeStatus.refetch();
    }
  },
});

const actionLabel = computed(() => {
  const status = runtimeStatus.data.value?.kernel.status;
  if (status === 'Running') return t('traffic.restart');
  if (status === 'Failed') return t('traffic.retry');
  return t('traffic.start');
});

async function runKernelAction() {
  actionStartedFromFailed =
    runtimeStatus.data.value?.kernel.status === 'Failed';
  failureNotificationVersionAtStart = failureNotificationVersion.value;
  await restartQuery.refetch();
}

function formatBytes(value: number) {
  if (value < 1024) return `${Math.round(value)} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let unitIndex = -1;
  let nextValue = value;
  do {
    nextValue /= 1024;
    unitIndex += 1;
  } while (nextValue >= 1024 && unitIndex < units.length - 1);
  return `${nextValue.toFixed(nextValue >= 10 ? 0 : 1)} ${units[unitIndex]}`;
}

defineOptions({ name: 'SystemMetricsPanel' });

export default __render(() => {
  const status = coreStatus.value;
  const StatusIcon = status.icon;
  const statusContent = (
    <>
      <Icon class={['text-sm', status.class]}>
        <StatusIcon />
      </Icon>
      <span class={status.class}>{status.label}</span>
    </>
  );
  return (
    <section class="rounded border border-outline-variant bg-surface-container p-3">
      <div class="mb-3 flex items-center justify-between text-on-surface">
        <div class="flex items-center gap-2">
          <Icon class="text-lg text-primary">
            <SpeedOutlined />
          </Icon>
          <span class="text-xs font-semibold uppercase tracking-wider">
            Traffic
          </span>
        </div>
        {status.href ? (
          <Tooltip content={status.tooltip}>
            <a
              aria-label={status.tooltip}
              href={status.href}
              class={[
                'flex h-6 items-center gap-1 rounded-full px-2',
                'bg-surface-container-high',
                'text-xs text-on-surface-variant outline-none',
                'transition-colors hover:bg-surface-container-highest',
                'focus-visible:ring-2 focus-visible:ring-tertiary',
              ]}
            >
              {statusContent}
            </a>
          </Tooltip>
        ) : (
          <div
            class={[
              'flex h-6 items-center gap-1 rounded-full px-2',
              'bg-surface-container-high',
              'text-xs text-on-surface-variant',
            ]}
          >
            {statusContent}
          </div>
        )}
      </div>
      <div class="space-y-3">
        <div class="grid grid-cols-2 gap-2">
          <div class="rounded bg-surface-container-high px-2 py-1.5">
            <div class="flex items-center gap-1 text-xs text-on-surface-variant">
              <Icon class="text-sm text-secondary">
                <ArrowDownwardOutlined />
              </Icon>
              Down
            </div>
            <p class="mt-1 font-mono text-xs text-on-surface">
              {formatBytes(traffic.value.down)}/s
            </p>
          </div>
          <div class="rounded bg-surface-container-high px-2 py-1.5">
            <div class="flex items-center gap-1 text-xs text-on-surface-variant">
              <Icon class="text-sm text-primary">
                <ArrowUpwardOutlined />
              </Icon>
              Up
            </div>
            <p class="mt-1 font-mono text-xs text-on-surface">
              {formatBytes(traffic.value.up)}/s
            </p>
          </div>
        </div>
        {runtimeStatus.data.value?.kernel.installed ? (
          <Button
            aria-label={actionLabel.value}
            block
            disabled={
              !runtimeStatus.data.value || restartQuery.isFetching.value
            }
            shape="rect"
            size="sm"
            variant={
              runtimeStatus.data.value.kernel.status === 'Failed'
                ? 'danger-ghost'
                : 'outline'
            }
            onClick={runKernelAction}
          >
            <Icon class="text-base">
              <RestartAltOutlined />
            </Icon>
            {actionLabel.value}
          </Button>
        ) : null}
      </div>
    </section>
  );
});
