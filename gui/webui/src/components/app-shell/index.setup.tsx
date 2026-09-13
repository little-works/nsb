import { __render } from '@/shared/helper';
import { h, useSlots } from 'vue';
import type { NavItem } from '../../pages/index/types';
import { type AppPageTypes } from '@/types';
import { Icon } from '@/components/icon';
import { FloatingDock } from '@/components/floating-dock';
import SystemMetricsPanel from './system-metrics-panel.setup';

export interface AppShellProps {
  navItems: NavItem[];
  selectedView: AppPageTypes;
}

const props = defineProps<AppShellProps>();
const slots = useSlots();

defineOptions({ name: 'AppShell' });

export default __render<AppShellProps>(() => {
  return (
    <div class="min-h-screen bg-background text-on-surface">
      <aside class="fixed left-0 top-0 z-sidebar hidden h-full w-60 flex-col border-r border-outline-variant bg-surface-dim py-4 md:flex">
        <div class="mb-8 px-4">
          <div class="flex items-center gap-3">
            <div class="flex h-10 w-10 items-center justify-center overflow-hidden rounded bg-[#e7eefe] p-1">
              <img
                src="/webui/nsb-logo.png"
                alt="NSB"
                class="h-full w-full object-contain dark:grayscale"
              />
            </div>
            <div>
              <h1 class="text-lg font-bold leading-none text-primary">
                Not Sing-Box
              </h1>
              <span class="mt-1 block text-xs font-bold tracking-wider text-secondary">
                v{__NSB_VERSION__}
              </span>
            </div>
          </div>
        </div>

        <nav class="flex-1 space-y-1 px-0">
          {props.navItems.map((item) => {
            const active = props.selectedView === item.key;
            const NavIcon = active ? item.activeIcon : item.icon;
            return (
              <a
                key={item.key}
                href={item.href}
                class={[
                  'flex h-11 w-full items-center gap-3 border-l-4 px-4 text-left transition-colors',
                  active
                    ? 'border-primary bg-surface-container-high text-primary'
                    : 'border-l-transparent text-on-surface-variant hover:bg-surface-container',
                ]}
              >
                <Icon class="text-xl">{h(NavIcon)}</Icon>
                <span
                  class={['text-sm leading-5', active ? 'font-semibold' : '']}
                >
                  {item.label}
                </span>
              </a>
            );
          })}
        </nav>

        <div class="mt-auto border-t border-outline-variant/50 px-4 pt-4">
          <SystemMetricsPanel />
        </div>
      </aside>

      <div class="md:ml-60">
        <div class="border-b border-outline-variant bg-surface px-4 md:hidden">
          <div class="flex items-center gap-2 overflow-x-auto">
            {props.navItems.map((item) => {
              const active = props.selectedView === item.key;
              return (
                <a
                  key={item.key}
                  href={item.href}
                  class={[
                    'flex h-8 items-center rounded-full border px-3 text-sm whitespace-nowrap',
                    active
                      ? 'border-primary bg-surface-container-high text-primary'
                      : 'border-outline-variant text-on-surface-variant',
                  ]}
                >
                  {item.label}
                </a>
              );
            })}
          </div>
        </div>

        <div>{slots.default?.()}</div>
      </div>
      <FloatingDock />
    </div>
  );
});
