import { __render } from '@/shared/helpter';
import {
  ArrowDownwardOutlined,
  ArrowUpwardOutlined,
  CheckCircleFilled,
  RadioButtonUncheckedFilled,
  SpeedOutlined,
} from '@vicons/material';
import { useAppSnapshot } from '@/store/app';
import { useScoreStreamData } from '@/hooks/use-score-stream';
import { computed } from 'vue';
import { Icon } from '@/components/icon';
const { traffic } = useScoreStreamData();
const appSnapshot = useAppSnapshot();
const coreStatus = computed(() => {
  const running = appSnapshot.data.value?.state.kernel.status === 'Running';
  return running
    ? { icon: CheckCircleFilled, label: 'Running', class: 'text-secondary' }
    : {
        icon: RadioButtonUncheckedFilled,
        label: 'Stopped',
        class: 'text-on-surface-variant',
      };
});
const mixedPort = computed(() => {
  const port = appSnapshot.data.value?.state.gui_config.mixed_port;
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
        <div class="flex h-6 items-center gap-1 rounded-full bg-surface-container-high px-2 text-xs text-on-surface-variant">
          <Icon class={['text-sm', status.class]}>
            <StatusIcon />
          </Icon>
          <span>{status.label}</span>
        </div>
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
