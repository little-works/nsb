import { __render } from '@/shared/helper';
import type { ProxyItem } from './types';
import { useI18n } from 'vue-i18n';

export interface ProxyNodeCardProps {
  item: ProxyItem;
  onSwitch?: () => void | Promise<void>;
}

const props = defineProps<ProxyNodeCardProps>();

function getLatencyClass(item: ProxyItem) {
  if (item.alive === false || item.latencyMs == null) {
    return 'text-error';
  }

  return item.latencyMs <= 450 ? 'text-success' : 'text-warn';
}

defineOptions({ name: 'ProxyNodeCard' });
const { t } = useI18n();

export default __render<ProxyNodeCardProps>(() => {
  return (
    <button
      type="button"
      disabled={props.item.active}
      class="
        grid h-20 w-full min-w-0 grid-cols-[minmax(0,1fr)_auto] items-stretch gap-2
        rounded border border-outline-variant bg-surface-container-lowest p-3
        text-left
        transition-all duration-200 ease-in-out hover:-translate-y-px hover:border-primary hover:bg-surface-container-low
        disabled:cursor-default disabled:hover:translate-y-0 disabled:hover:border-outline-variant disabled:hover:bg-surface-container-lowest
      "
      onClick={props.onSwitch}
    >
      <div class="flex min-w-0 items-center gap-2">
        <div
          class={[
            'shrink-0 rounded-full transition-shadow',
            props.item.active
              ? 'h-2 w-2 bg-secondary ring-2 ring-secondary/30 ring-offset-1 ring-offset-surface-container-lowest'
              : 'h-1 w-1 bg-outline',
          ]}
        ></div>
        <span class="min-w-0 line-clamp-3 text-sm font-medium leading-4 text-on-surface">
          {props.item.name}
        </span>
      </div>
      <div class="flex min-w-0 flex-col items-end justify-between">
        <span
          class={[
            'font-mono text-xs leading-4 whitespace-nowrap',
            getLatencyClass(props.item),
          ]}
        >
          {props.item.latency}
        </span>
        <span
          class="
            flex h-5 max-w-full items-center truncate rounded bg-surface-container-high px-1.5
            font-mono text-xs uppercase tracking-tighter whitespace-nowrap text-on-surface-variant
          "
        >
          {props.item.tag || t('home.unknown')}
        </span>
      </div>
    </button>
  );
});
