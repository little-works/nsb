import { fetchState } from '@/api/app';
import { useClientQuery } from '@/hooks/use-client-query';
import { type AppPageTypes, type AppSnapshot } from '@/types';
import { computed, type ComputedRef } from 'vue';
import { usePageContext } from 'vike-vue/usePageContext';
import { resolveAppPageType } from '@/shared/page';

const appSnapshotQueryKey = ['appSnapshot'];

export function useAppSnapshot() {
  return useClientQuery({
    queryKey: appSnapshotQueryKey,
    queryFn: fetchState,
    staleTime: 30 * 1000,
    refetchOnWindowFocus: true,
  });
}

export function useAppPageType(): ComputedRef<AppPageTypes> {
  const context = usePageContext();
  return computed(() => resolveAppPageType(context.urlPathname));
}
