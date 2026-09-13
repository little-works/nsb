import { __render } from '@/shared/helper';
import { Button } from '@/components/button';
import { Checkbox } from '@/components/checkbox';
import { Icon } from '@/components/icon';
import {
  ArrowDownwardOutlined,
  ArrowUpwardOutlined,
  SearchOutlined,
  SortOutlined,
} from '@vicons/material';
import type { ProxyMode, ProxySortMode } from './types';
import { useI18n } from 'vue-i18n';

const proxyModeOptions: Array<{ labelKey: string; value: ProxyMode }> = [
  { labelKey: 'home.global', value: 'global' },
  { labelKey: 'home.rule', value: 'rule' },
  { labelKey: 'home.direct', value: 'direct' },
];

export interface OverviewToolbarProps {
  searchKeyword: string;
  hideUnavailableNodes: boolean;
  kernelRunning: boolean;
  proxyMode: ProxyMode;
  proxyModeChanging: boolean;
  sortMode: ProxySortMode;
  onSearchKeywordChange?: (value: string) => void;
  onHideUnavailableNodesChange?: (value: boolean) => void;
  onSortModeCycle?: () => void;
  onProxyModeChange?: (mode: ProxyMode) => void | Promise<void>;
}

const props = defineProps<OverviewToolbarProps>();

function getSortLabel(t: (key: string) => string) {
  if (props.sortMode === 'asc') {
    return t('home.speedAsc');
  }
  if (props.sortMode === 'desc') {
    return t('home.speedDesc');
  }
  return t('home.sortBySpeed');
}

function getSortIcon() {
  if (props.sortMode === 'asc') {
    return ArrowUpwardOutlined;
  }
  if (props.sortMode === 'desc') {
    return ArrowDownwardOutlined;
  }
  return SortOutlined;
}

defineOptions({ name: 'OverviewToolbar' });
const { t } = useI18n();

export default __render<OverviewToolbarProps>(() => {
  const SortIcon = getSortIcon();

  return (
    <div class="sticky top-0 z-page-header h-36 bg-background py-3">
      <div class="mx-auto max-w-6xl space-y-3 px-4">
        <div class="relative">
          <Icon class="pointer-events-none absolute left-4 top-1/2 -translate-y-1/2 text-on-surface-variant">
            <SearchOutlined />
          </Icon>
          <input
            class="h-11 w-full rounded-full border border-outline-variant bg-surface-container-lowest pl-12 pr-4 text-sm leading-5 text-on-surface focus:border-primary focus:outline-none"
            placeholder={t('home.search')}
            type="text"
            value={props.searchKeyword}
            onInput={(event) => {
              props.onSearchKeywordChange?.(
                (event.target as HTMLInputElement).value,
              );
            }}
          />
        </div>

        <div class="flex h-14 items-center rounded border border-outline-variant bg-surface-container-low px-3">
          <div class="flex items-center gap-4 overflow-x-auto">
            <div class="flex h-8 shrink-0 items-center rounded-full bg-surface-container-high p-0.5">
              {proxyModeOptions.map((option) => (
                <Button
                  key={option.value}
                  disabled={!props.kernelRunning || props.proxyModeChanging}
                  shape="pill"
                  size="xs"
                  variant={props.proxyMode === option.value ? 'solid' : 'ghost'}
                  onClick={() => props.onProxyModeChange?.(option.value)}
                >
                  {t(option.labelKey)}
                </Button>
              ))}
            </div>
            <Button
              shape="pill"
              size="sm"
              variant={props.sortMode === 'none' ? 'subtle' : 'solid'}
              onClick={props.onSortModeCycle}
            >
              <Icon class="text-lg">
                <SortIcon />
              </Icon>
              <span class="text-xs font-medium">{getSortLabel(t)}</span>
            </Button>
            <label class="flex items-center space-x-2">
              <Checkbox
                checked={props.hideUnavailableNodes}
                onChange={props.onHideUnavailableNodesChange}
              />
              <span class="text-xs font-medium text-on-surface-variant">
                {t('home.hideUnavailable')}
              </span>
            </label>
          </div>
        </div>
      </div>
    </div>
  );
});
