import { __render } from '@/shared/helpter';
import { IconButton } from '@/components/button';
import { Icon } from '@/components/icon';
import {
  BoltOutlined,
  ExpandMoreOutlined,
  PublicOutlined,
} from '@vicons/material';
import ProxyNodeCard from './proxy-node-card.setup';
import type { ProxyGroup } from './types';
import { useI18n } from 'vue-i18n';

export interface ProxyGroupSectionProps {
  anchorId: string;
  group: ProxyGroup;
  expanded: boolean;
  onToggle?: () => void;
  onSwitchProxy?: (group: string, proxy: string) => void | Promise<void>;
  onTestProxyGroup?: (group: string) => void | Promise<void>;
}

const props = defineProps<ProxyGroupSectionProps>();

defineOptions({ name: 'ProxyGroupSection' });
const { t } = useI18n();

export default __render<ProxyGroupSectionProps>(() => {
  return (
    <section
      id={props.anchorId}
      class="scroll-mt-40 rounded bg-surface-container-lowest"
    >
      <div
        class={[
          // 'bg-surface-container-low',
          'bg-background',
          props.expanded ? 'sticky top-36 z-sticky' : '',
        ]}
        onClick={props.onToggle}
      >
        <div
          class={[
            'bg-surface-container-low flex w-full items-center justify-between p-4',
            'shadow-sm border border-outline-variant',
            props.expanded ? 'rounded-t' : 'rounded',
          ]}
        >
          <div class="flex min-w-0 flex-1 items-center space-x-3 text-left">
            <Icon class="shrink-0 text-primary">
              <PublicOutlined />
            </Icon>
            <div class="min-w-0">
              <div class="flex min-w-0 items-center space-x-3">
                <h2 class="truncate text-lg font-semibold leading-6 text-on-surface">
                  {props.group.title}
                </h2>
                <span class="flex h-5 shrink-0 items-center rounded-full bg-surface-variant px-2 text-xs font-medium whitespace-nowrap text-on-surface-variant">
                  {props.group.type}
                </span>
                <span class="flex h-5 shrink-0 items-center rounded-full bg-surface-container-high px-2 text-xs font-medium whitespace-nowrap uppercase text-on-surface-variant">
                  {props.group.groupType}
                </span>
              </div>
              <div class="mt-1 text-xs leading-4 text-on-surface-variant">
                {t('home.current', { name: props.group.active || '--' })}
              </div>
            </div>
          </div>

          <IconButton
            class="ml-1"
            iconClass={[
              'transition-transform',
              props.expanded ? 'rotate-180' : '',
            ]}
            tooltip={t(props.expanded ? 'home.collapse' : 'home.expand')}
          >
            <ExpandMoreOutlined />
          </IconButton>
          <IconButton
            class="ml-1"
            tooltip={t('home.testLatency')}
            onClick={async (e) => {
              e.stopPropagation();
              if (!props.expanded) {
                props.onToggle?.();
              }
              await props.onTestProxyGroup?.(props.group.title);
            }}
          >
            <BoltOutlined />
          </IconButton>
        </div>
      </div>

      {props.expanded ? (
        <div class="rounded-b bg-surface p-4 border border-outline-variant border-t-0">
          <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {props.group.items.map((item) => (
              <div key={`${props.group.title}-${item.name}`}>
                <ProxyNodeCard
                  item={item}
                  onSwitch={
                    item.active
                      ? undefined
                      : () =>
                          props.onSwitchProxy?.(props.group.title, item.name)
                  }
                />
              </div>
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
});
