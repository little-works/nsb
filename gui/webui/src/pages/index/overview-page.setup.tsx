import { __render } from '@/shared/helpter';
import { IconButton } from '@/components/button';
import { Page, PageContent, PageHeader } from '@/components/page-content';
import { useFloatingDockStore } from '@/store/floating-dock';
import { RefreshOutlined } from '@vicons/material';
import { useLocalStorage } from '@vueuse/core';
import {
  computed,
  nextTick,
  onActivated,
  onBeforeUnmount,
  onDeactivated,
  onMounted,
  ref,
  watch,
} from 'vue';
import Feedback from './feedback.setup';
import GroupDockContent from './group-dock-content.setup';
import Toolbar from './toolbar.setup';
import ProxyGroupSection from './proxy-group-section.setup';
import type { ProxyGroup, ProxyMode, ProxySortMode } from './types';
import { useI18n } from 'vue-i18n';

const EXPANDED_PROXY_GROUPS_KEY = 'nsb-expanded-proxy-groups';

export interface OverviewPageProps {
  groups: ProxyGroup[];
  loading: boolean;
  errorMessage: string;
  kernelRunning: boolean;
  searchKeyword: string;
  hideUnavailableNodes: boolean;
  sortMode: ProxySortMode;
  proxyMode: ProxyMode;
  proxyModeChanging: boolean;
  onHideUnavailableNodesChange?: (value: boolean) => void;
  onSearchKeywordChange?: (value: string) => void;
  onSortModeCycle?: () => void;
  onProxyModeChange?: (mode: ProxyMode) => void | Promise<void>;
  onRefresh?: () => void | Promise<void>;
  onSwitchProxy?: (group: string, proxy: string) => void | Promise<void>;
  onTestProxyGroup?: (group: string) => void | Promise<void>;
}

const props = defineProps<OverviewPageProps>();

const expandedGroups = useLocalStorage<Record<string, boolean>>(
  EXPANDED_PROXY_GROUPS_KEY,
  {},
  { initOnMounted: true },
);

const searching = computed(() => props.searchKeyword.trim().length > 0);
const activeGroupTitle = ref('');
const floatingDockStore = useFloatingDockStore();
let scrollContainer: HTMLElement | null = null;
let unregisterDockContent: (() => void) | null = null;
let homeDockActive = false;

function isGroupExpanded(groupTitle: string, index: number) {
  return searching.value || (expandedGroups.value[groupTitle] ?? index === 0);
}

function toggleGroup(groupTitle: string, index: number) {
  if (searching.value) {
    return;
  }

  expandedGroups.value[groupTitle] = !isGroupExpanded(groupTitle, index);
}

function getGroupAnchorId(groupTitle: string) {
  return `proxy-group-${encodeURIComponent(groupTitle)}`;
}

function scrollToGroup(groupTitle: string) {
  expandedGroups.value[groupTitle] = true;
  activeGroupTitle.value = groupTitle;
  void nextTick(() => {
    document
      .getElementById(getGroupAnchorId(groupTitle))
      ?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  });
}

function updateActiveGroup() {
  if (!scrollContainer || props.groups.length === 0) {
    activeGroupTitle.value = '';
    return;
  }

  const containerTop = scrollContainer.getBoundingClientRect().top;
  let nextActiveGroup = props.groups[0].title;

  for (const group of props.groups) {
    const section = document.getElementById(getGroupAnchorId(group.title));
    if (!section) {
      continue;
    }
    if (section.getBoundingClientRect().top - containerTop <= 160) {
      nextActiveGroup = group.title;
      continue;
    }
    break;
  }

  activeGroupTitle.value = nextActiveGroup;
}

function setupGroupScrollSpy() {
  scrollContainer?.removeEventListener('scroll', updateActiveGroup);
  void nextTick(() => {
    if (!homeDockActive) {
      return;
    }
    scrollContainer = document.querySelector('main');
    scrollContainer?.addEventListener('scroll', updateActiveGroup, {
      passive: true,
    });
    updateActiveGroup();
  });
}

function cleanupGroupScrollSpy() {
  scrollContainer?.removeEventListener('scroll', updateActiveGroup);
  scrollContainer = null;
}

function activateHomeDock() {
  if (homeDockActive) {
    return;
  }

  homeDockActive = true;
  unregisterDockContent = floatingDockStore.register(() => (
    <GroupDockContent
      activeGroupTitle={activeGroupTitle.value}
      groups={props.groups}
      onGroupSelect={scrollToGroup}
    />
  ));
  setupGroupScrollSpy();
}

function deactivateHomeDock() {
  homeDockActive = false;
  cleanupGroupScrollSpy();
  unregisterDockContent?.();
  unregisterDockContent = null;
}

onMounted(activateHomeDock);
onActivated(activateHomeDock);
onDeactivated(deactivateHomeDock);
onBeforeUnmount(deactivateHomeDock);
watch(
  () => props.groups.map((group) => group.title).join('\u0000'),
  () => {
    if (homeDockActive) {
      setupGroupScrollSpy();
    }
  },
);

defineOptions({ name: 'OverviewPage' });

export default __render<OverviewPageProps>(() => {
  const { t } = useI18n();
  return (
    <Page>
      {{
        header: () => {
          return (
            <PageHeader title={t('home.title')}>
              {{
                actions: () => (
                  <IconButton
                    title={t('home.refresh')}
                    onClick={props.onRefresh}
                  >
                    <RefreshOutlined />
                  </IconButton>
                ),
              }}
            </PageHeader>
          );
        },
        default: () => (
          <>
            <Toolbar
              hideUnavailableNodes={props.hideUnavailableNodes}
              kernelRunning={props.kernelRunning}
              onHideUnavailableNodesChange={props.onHideUnavailableNodesChange}
              onSearchKeywordChange={props.onSearchKeywordChange}
              onSortModeCycle={props.onSortModeCycle}
              onProxyModeChange={props.onProxyModeChange}
              proxyMode={props.proxyMode}
              proxyModeChanging={props.proxyModeChanging}
              searchKeyword={props.searchKeyword}
              sortMode={props.sortMode}
            />

            <PageContent class="space-y-4 pb-24">
              <Feedback
                errorMessage={props.errorMessage}
                hasGroups={props.groups.length > 0}
                kernelRunning={props.kernelRunning}
                loading={props.loading}
              />

              {props.groups.map((group, index) => {
                const expanded = isGroupExpanded(group.title, index);
                return (
                  <ProxyGroupSection
                    anchorId={getGroupAnchorId(group.title)}
                    key={group.title}
                    expanded={expanded}
                    group={group}
                    onSwitchProxy={props.onSwitchProxy}
                    onTestProxyGroup={props.onTestProxyGroup}
                    onToggle={() => toggleGroup(group.title, index)}
                  />
                );
              })}
            </PageContent>
          </>
        ),
        overlay: () => (
          <>
            <div
              class="pointer-events-none fixed inset-0 z-[-1] opacity-[0.03]"
              style="background-image: radial-gradient(#0058be 0.5px, transparent 0.5px); background-size: 24px 24px;"
            ></div>
          </>
        ),
      }}
    </Page>
  );
});
