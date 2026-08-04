import { __render } from '@/shared/helpter';
import {
  ArrowDownwardOutlined,
  ArrowUpwardOutlined,
  CheckCircleFilled,
  RadioButtonUncheckedFilled,
  SpeedOutlined,
  WarningAmberOutlined,
} from '@vicons/material';
import { Tooltip } from '@/components/tooltip';
import { useRuntimeSettings, useRuntimeStatus } from '@/store/app';
import { useScoreStreamData } from '@/hooks/use-score-stream';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Icon } from '@/components/icon';
const { traffic } = useScoreStreamData();
const runtimeStatus = useRuntimeStatus();
const runtimeSettings = useRuntimeSettings();
const { t } = useI18n();
const coreStatus = computed(() => {
  const installed = runtimeStatus.data.value?.kernel.installed ?? true;
  const running = runtimeStatus.data.value?.kernel.status === 'Running';
  if (!installed) {
    return {
      icon: WarningAmberOutlined,
      label: t('traffic.noCore'),
      class: 'text-tertiary',
      href: '/webui/settings',
      tooltip: t('traffic.noCoreAction'),
    };
  }
  return running
    ? {
        icon: CheckCircleFilled,
        label: t('traffic.running'),
        class: 'text-secondary',
        href: undefined,
        tooltip: undefined,
      }
    : {
        icon: RadioButtonUncheckedFilled,
        label: t('traffic.stopped'),
        class: 'text-on-surface-variant',
        href: undefined,
        tooltip: undefined,
      };
});
const mixedPort = computed(() => {
  const port = runtimeSettings.data.value?.mixed_port;
  return port == null ? '--' : String(port);
});

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
      <span>{status.label}</span>
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
              class="flex h-6 items-center gap-1 rounded-full bg-surface-container-high px-2
                text-xs text-on-surface-variant outline-none
                transition-colors hover:bg-surface-container-highest
                focus-visible:ring-2 focus-visible:ring-tertiary"
            >
              {statusContent}
            </a>
          </Tooltip>
        ) : (
          <div class="flex h-6 items-center gap-1 rounded-full bg-surface-container-high px-2 text-xs text-on-surface-variant">
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
        <div class="flex h-6 items-center justify-between rounded bg-surface-container-high px-2 text-xs text-on-surface-variant">
          <span>Mixed Port</span>
          <span class="font-mono text-on-surface">{mixedPort.value}</span>
        </div>
      </div>
    </section>
  );
});
