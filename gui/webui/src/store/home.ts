import {
  getScoreConfig,
  getScoreProxies,
  selectScoreProxy,
  setScoreMode,
  startScoreLatencyTest,
} from '@/api/score';
import { useClientQuery } from '@/hooks/use-client-query';
import { useScoreStreamData } from '@/hooks/use-score-stream';
import { useRuntimeStatus } from '@/store/app';
import type {
  CoreApiProxies,
  CoreApiProxy,
  CoreApiProxyLatencyResult,
  RuntimeStatus,
} from '@/types';
import { useLocalStorage } from '@vueuse/core';
import { defineStore } from 'pinia';
import { computed, ref, watch } from 'vue';
import { toast } from '@/components/toast';
import type {
  ProxyGroup,
  ProxyItem,
  ProxyMode,
  ProxySortMode,
} from '@/pages/index/types';
import { i18n } from '@/i18n';

const HIDE_UNAVAILABLE_NODES_KEY = 'nsb-hide-unavailable-nodes';
const PROXY_SORT_MODE_KEY = 'nsb-proxy-sort-mode';

function isKernelReady(snapshot: RuntimeStatus) {
  return snapshot.kernel.status === 'Running';
}

function getLatencyMs(proxy?: CoreApiProxy) {
  const history = proxy?.history || [];
  for (let index = history.length - 1; index >= 0; index -= 1) {
    if (history[index].delay > 0) {
      return history[index].delay;
    }
  }
  return undefined;
}

function formatLatency(latencyMs?: number) {
  return latencyMs == null ? '--' : `${latencyMs} ms`;
}

function applyRuntimeState(
  groups: ProxyGroup[],
  {
    latencyOverrides = {},
  }: {
    latencyOverrides?: Record<string, CoreApiProxyLatencyResult>;
  } = {},
) {
  return groups.map((group) => ({
    ...group,
    items: group.items.map((item) => {
      const override = latencyOverrides[item.name];
      const latencyMs = override
        ? (override.latencyMs ?? undefined)
        : item.latencyMs;
      return {
        ...item,
        alive: override?.alive ?? item.alive,
        latency: formatLatency(latencyMs),
        latencyMs,
      };
    }),
  }));
}

function mapProxyGroups(payload: CoreApiProxies): ProxyGroup[] {
  const groupNames = new Set(
    Object.entries(payload.proxies)
      .filter(([, proxy]) => Array.isArray(proxy.all))
      .map(([name]) => name),
  );

  return Object.entries(payload.proxies)
    .filter(([, proxy]) => Array.isArray(proxy.all) && proxy.all.length > 0)
    .map(([name, proxy]) => ({
      title: name,
      type: `${proxy.all.length} Nodes`,
      groupType: proxy.type.toUpperCase(),
      active: proxy.now,
      items: proxy.all
        .map((itemName): ProxyItem | null => {
          const item = payload.proxies[itemName];
          if (!item) {
            return null;
          }
          const latencyMs = getLatencyMs(item);
          return {
            name: itemName,
            latency: formatLatency(latencyMs),
            latencyMs,
            active: itemName === proxy.now,
            tag: item.type.toUpperCase(),
            alive: item.alive,
          };
        })
        .filter((item): item is ProxyItem => item != null),
    }))
    .filter((group) => group.items.length > 0)
    .sort((left, right) => {
      if (left.title === 'GLOBAL' && right.title !== 'GLOBAL') {
        return 1;
      }
      if (left.title !== 'GLOBAL' && right.title === 'GLOBAL') {
        return -1;
      }

      const leftGroupItemCount = left.items.filter((item) =>
        groupNames.has(item.name),
      ).length;
      const rightGroupItemCount = right.items.filter((item) =>
        groupNames.has(item.name),
      ).length;
      const ratioDifference =
        rightGroupItemCount * left.items.length -
        leftGroupItemCount * right.items.length;

      return ratioDifference || left.title.localeCompare(right.title);
    });
}

function matchesSearch(value: string, keyword: string) {
  return value.toLowerCase().includes(keyword);
}

function matchesProxyItemSearch(item: ProxyItem, keyword: string) {
  return (
    matchesSearch(item.name, keyword) ||
    matchesSearch(item.tag || '', keyword) ||
    matchesSearch(item.latency, keyword)
  );
}

function isProxyItemAvailable(item: ProxyItem) {
  return item.alive !== false && item.latencyMs != null;
}

function compareProxyItemsByLatency(
  left: ProxyItem,
  right: ProxyItem,
  sortMode: Exclude<ProxySortMode, 'none'>,
) {
  if (left.latencyMs == null && right.latencyMs == null) {
    return 0;
  }
  if (left.latencyMs == null) {
    return 1;
  }
  if (right.latencyMs == null) {
    return -1;
  }

  return sortMode === 'asc'
    ? left.latencyMs - right.latencyMs
    : right.latencyMs - left.latencyMs;
}

function normalizeSortMode(value: string): ProxySortMode {
  return value === 'asc' || value === 'desc' ? value : 'none';
}

function normalizeProxyMode(value: string | undefined): ProxyMode {
  return value === 'global' || value === 'direct' ? value : 'rule';
}

export const useHomeStore = defineStore('home', () => {
  const runtimeStatus = useRuntimeStatus();
  const { latencyResults } = useScoreStreamData();
  let switchingProxy = false;
  const switchingProxyMode = ref(false);
  const searchKeyword = ref('');
  const hideUnavailableNodes = useLocalStorage(
    HIDE_UNAVAILABLE_NODES_KEY,
    false,
    { initOnMounted: true },
  );
  const sortMode = useLocalStorage<ProxySortMode>(PROXY_SORT_MODE_KEY, 'none', {
    initOnMounted: true,
  });
  const actionErrorMessage = ref('');

  watch(i18n.global.locale, () => {
    actionErrorMessage.value = '';
  });

  const kernelRunning = computed(
    () => runtimeStatus.data.value?.kernel.status === 'Running',
  );
  const kernelInstalled = computed(
    () => runtimeStatus.data.value?.kernel.installed ?? true,
  );

  const proxyGroupsQuery = useClientQuery(
    computed(() => ({
      queryKey: [
        'overviewProxyGroups',
        runtimeStatus.data.value?.kernel.status ?? '',
      ],
      queryFn: async () => {
        const snapshot =
          runtimeStatus.data.value ?? (await runtimeStatus.refetch()).data;
        if (!snapshot) {
          throw new Error(i18n.global.t('home.loadFailed'));
        }

        if (!isKernelReady(snapshot)) {
          return [] as ProxyGroup[];
        }

        const proxies = await getScoreProxies();
        return mapProxyGroups(proxies);
      },
      staleTime: 15 * 1000,
      refetchOnWindowFocus: true,
    })),
  );

  const proxyModeQuery = useClientQuery(
    computed(() => ({
      queryKey: [
        'overviewProxyMode',
        runtimeStatus.data.value?.kernel.status ?? '',
      ],
      queryFn: async () => {
        const snapshot =
          runtimeStatus.data.value ?? (await runtimeStatus.refetch()).data;
        if (!snapshot || !isKernelReady(snapshot)) {
          return 'rule' as ProxyMode;
        }

        const config = await getScoreConfig();
        return normalizeProxyMode(config.mode);
      },
      staleTime: 15 * 1000,
      refetchOnWindowFocus: true,
    })),
  );

  const displayProxyGroups = computed(() =>
    applyRuntimeState(proxyGroupsQuery.data.value ?? [], {
      latencyOverrides: latencyResults.value,
    }),
  );

  const searchedProxyGroups = computed(() => {
    const keyword = searchKeyword.value.trim().toLowerCase();
    if (!keyword) {
      return displayProxyGroups.value;
    }

    return displayProxyGroups.value
      .map((group) => {
        return {
          ...group,
          items: group.items.filter((item) =>
            matchesProxyItemSearch(item, keyword),
          ),
        };
      })
      .filter((group) => group.items.length > 0);
  });

  const filteredProxyGroups = computed(() => {
    if (!hideUnavailableNodes.value) {
      return searchedProxyGroups.value;
    }

    return searchedProxyGroups.value
      .map((group) => ({
        ...group,
        items: group.items.filter(isProxyItemAvailable),
      }))
      .filter((group) => group.items.length > 0);
  });

  const normalizedSortMode = computed(() => normalizeSortMode(sortMode.value));
  const proxyMode = computed(() =>
    normalizeProxyMode(proxyModeQuery.data.value),
  );

  const proxyGroups = computed(() => {
    const activeSortMode = normalizedSortMode.value;
    if (activeSortMode === 'none') {
      return filteredProxyGroups.value;
    }

    return filteredProxyGroups.value.map((group) => ({
      ...group,
      items: [...group.items].sort((left, right) =>
        compareProxyItemsByLatency(left, right, activeSortMode),
      ),
    }));
  });

  const loading = computed(
    () => runtimeStatus.isFetching.value || proxyGroupsQuery.isFetching.value,
  );

  const errorMessage = computed(() => {
    if (actionErrorMessage.value) {
      return actionErrorMessage.value;
    }
    if (runtimeStatus.error.value instanceof Error) {
      return runtimeStatus.error.value.message;
    }
    if (proxyGroupsQuery.error.value instanceof Error) {
      return proxyGroupsQuery.error.value.message;
    }
    if (proxyModeQuery.error.value instanceof Error) {
      return proxyModeQuery.error.value.message;
    }
    return '';
  });

  function setSearchKeyword(value: string) {
    searchKeyword.value = value;
  }

  function setHideUnavailableNodes(value: boolean) {
    hideUnavailableNodes.value = value;
  }

  function cycleSortMode() {
    const currentSortMode = normalizedSortMode.value;
    if (currentSortMode === 'none') {
      sortMode.value = 'asc';
      return;
    }

    if (currentSortMode === 'asc') {
      sortMode.value = 'desc';
      return;
    }

    sortMode.value = 'none';
  }

  async function refreshProxyGroups() {
    try {
      actionErrorMessage.value = '';
      const result = await proxyGroupsQuery.refetch();
      if (result.error) {
        throw result.error;
      }
    } catch (error) {
      actionErrorMessage.value =
        error instanceof Error
          ? error.message
          : i18n.global.t('home.refreshFailed');
    }
  }

  async function setProxyMode(mode: ProxyMode) {
    if (switchingProxyMode.value || mode === proxyMode.value) {
      return;
    }

    try {
      const snapshot =
        runtimeStatus.data.value ?? (await runtimeStatus.refetch()).data;
      if (!snapshot || !isKernelReady(snapshot)) {
        throw new Error(i18n.global.t('home.kernelStopped'));
      }

      actionErrorMessage.value = '';
      switchingProxyMode.value = true;
      await setScoreMode(mode);
      await proxyModeQuery.refetch();
    } catch (error) {
      actionErrorMessage.value =
        error instanceof Error
          ? error.message
          : i18n.global.t('home.modeFailed');
    } finally {
      switchingProxyMode.value = false;
    }
  }

  async function switchProxy(group: string, proxy: string) {
    if (switchingProxy) {
      return;
    }

    try {
      switchingProxy = true;
      const snapshot =
        runtimeStatus.data.value ?? (await runtimeStatus.refetch()).data;
      if (!snapshot || !isKernelReady(snapshot)) {
        throw new Error(i18n.global.t('home.kernelStopped'));
      }

      actionErrorMessage.value = '';

      await selectScoreProxy(group, proxy);
      await proxyGroupsQuery.refetch();
    } catch (error) {
      actionErrorMessage.value =
        error instanceof Error
          ? error.message
          : i18n.global.t('home.switchFailed');
    } finally {
      switchingProxy = false;
    }
  }

  async function testProxyGroup(groupTitle: string) {
    try {
      const snapshot =
        runtimeStatus.data.value ?? (await runtimeStatus.refetch()).data;
      if (!snapshot || !isKernelReady(snapshot)) {
        throw new Error(i18n.global.t('home.kernelStopped'));
      }

      const targetGroup = displayProxyGroups.value.find(
        (group) => group.title === groupTitle,
      );
      if (!targetGroup) {
        throw new Error(i18n.global.t('home.groupNotFound'));
      }

      actionErrorMessage.value = '';
      const accepted = await startScoreLatencyTest(
        targetGroup.items.map((item) => item.name),
      );
      if (!accepted) {
        toast.info({ title: i18n.global.t('home.latencyInProgress') });
      }
    } catch (error) {
      actionErrorMessage.value =
        error instanceof Error
          ? error.message
          : i18n.global.t('home.latencyFailed');
    }
  }

  return {
    errorMessage,
    hideUnavailableNodes,
    kernelInstalled,
    kernelRunning,
    loading,
    proxyMode,
    proxyModeChanging: switchingProxyMode,
    proxyGroups,
    searchKeyword,
    sortMode: normalizedSortMode,
    cycleSortMode,
    refreshProxyGroups,
    setHideUnavailableNodes,
    setProxyMode,
    setSearchKeyword,
    switchProxy,
    testProxyGroup,
  };
});
