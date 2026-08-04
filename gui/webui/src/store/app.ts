import { fetchRuntime, fetchSettings, listProfiles } from '@/api/app';
import { useClientQuery } from '@/hooks/use-client-query';
import { type AppPageTypes } from '@/types';
import { computed, type ComputedRef } from 'vue';
import { usePageContext } from 'vike-vue/usePageContext';
import { resolveAppPageType } from '@/shared/page';

const runtimeQueryKey = ['runtime'];
const settingsQueryKey = ['settings'];
const profilesQueryKey = ['profiles'];

export function useRuntimeStatus() {
  return useClientQuery({
    queryKey: runtimeQueryKey,
    queryFn: fetchRuntime,
    staleTime: 30 * 1000,
    refetchOnWindowFocus: true,
  });
}

export function useRuntimeSettings() {
  return useClientQuery({
    queryKey: settingsQueryKey,
    queryFn: fetchSettings,
    staleTime: 30 * 1000,
    refetchOnWindowFocus: true,
  });
}

export function useProfiles() {
  return useClientQuery({
    queryKey: profilesQueryKey,
    queryFn: listProfiles,
    staleTime: 30 * 1000,
    refetchOnWindowFocus: true,
  });
}

export function useAppPageType(): ComputedRef<AppPageTypes> {
  const context = usePageContext();
  return computed(() => resolveAppPageType(context.urlPathname));
}
