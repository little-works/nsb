import { __render } from '@/shared/helpter';
import { Button } from '@/components/button';
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

export default __render<ProxyNodeCardProps>(() => {
  const { t } = useI18n();
  return (
    <div class="flex flex-col rounded border border-outline-variant bg-surface-container-lowest p-3 transition-all duration-200 ease-in-out hover:-translate-y-px">
      <div class="mb-2 flex items-start justify-between">
        <div class="flex items-center space-x-2">
          <div
            class={[
              'h-1.5 w-1.5 rounded-full',
              props.item.active ? 'bg-secondary' : 'bg-outline',
            ]}
          ></div>
          <span class="max-w-30 truncate text-sm font-medium leading-5 text-on-surface">
            {props.item.name}
          </span>
        </div>
        <span
          class={[
            'flex h-5 shrink-0 items-center font-mono text-xs leading-4 whitespace-nowrap',
            getLatencyClass(props.item),
          ]}
        >
          {props.item.latency}
        </span>
      </div>
      <div class="mt-auto flex items-center justify-between">
        <span class="flex h-5 items-center rounded bg-surface-container-high px-1.5 font-mono text-xs uppercase tracking-tighter whitespace-nowrap text-on-surface-variant">
          {props.item.tag || t('home.unknown')}
        </span>
        <Button
          shape="rect"
          size="xs"
          variant={props.item.active ? 'solid' : 'outline'}
          onClick={props.onSwitch}
          class="font-bold uppercase"
        >
          {t(props.item.active ? 'home.active' : 'home.switch')}
        </Button>
      </div>
    </div>
  );
});
