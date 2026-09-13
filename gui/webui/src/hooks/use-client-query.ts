import { useMountedOrActivated } from '@/hooks/use-mounted-or-activated';
import { computed, unref, type Ref } from 'vue';
import {
  useQuery,
  type DefinedInitialQueryOptions,
  type QueryClient,
  type UndefinedInitialQueryOptions,
  type UseQueryDefinedReturnType,
  type UseQueryOptions,
  type UseQueryReturnType,
  type QueryOptions,
} from '@tanstack/vue-query';
import type { DefaultError, QueryKey } from '@tanstack/query-core';

type ClientQueryOptions<
  TQueryFnData,
  TError,
  TData,
  TQueryKey extends QueryKey,
> =
  | UseQueryOptions<TQueryFnData, TError, TData, TQueryFnData, TQueryKey>
  | Ref<UseQueryOptions<TQueryFnData, TError, TData, TQueryFnData, TQueryKey>>;

export function useClientQuery<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
>(
  options:
    | DefinedInitialQueryOptions<TQueryFnData, TError, TData, TQueryKey>
    | Ref<DefinedInitialQueryOptions<TQueryFnData, TError, TData, TQueryKey>>,
  queryClient?: QueryClient,
): UseQueryDefinedReturnType<TData, TError>;

export function useClientQuery<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
>(
  options:
    | UndefinedInitialQueryOptions<TQueryFnData, TError, TData, TQueryKey>
    | Ref<UndefinedInitialQueryOptions<TQueryFnData, TError, TData, TQueryKey>>,
  queryClient?: QueryClient,
): UseQueryReturnType<TData, TError>;

export function useClientQuery<
  TQueryFnData = unknown,
  TError = DefaultError,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
>(
  options: ClientQueryOptions<TQueryFnData, TError, TData, TQueryKey>,
  queryClient?: QueryClient,
): UseQueryReturnType<TData, TError>;

export function useClientQuery<
  TQueryFnData,
  TError = DefaultError,
  TData = TQueryFnData,
  TQueryKey extends QueryKey = QueryKey,
>(
  options: ClientQueryOptions<TQueryFnData, TError, TData, TQueryKey>,
  queryClient?: QueryClient,
):
  UseQueryReturnType<TData, TError> | UseQueryDefinedReturnType<TData, TError> {
  const query = useQuery(
    computed(() => {
      const resolvedOptions = unref(options);

      if (import.meta.env.SSR) {
        return {
          ...resolvedOptions,
          refetchOnMount: false,
          queryFn: async () => null as TQueryFnData,
        };
      }

      const clientOptions = {
        refetchOnMount: 'always' as const,
        ...resolvedOptions,
      };
      clientOptions.refetchOnMount ??= 'always';
      return clientOptions;
    }),
    queryClient,
  );

  let hasMounted = false;
  useMountedOrActivated(() => {
    if (!hasMounted) {
      hasMounted = true;
      return;
    }

    if (query.isFetching.value) return;

    const resolvedOptions = unref(options) as QueryOptions;
    if (resolvedOptions.enabled === false) return;

    const refetchOnMount = resolvedOptions.refetchOnMount ?? 'always';
    if (
      refetchOnMount === 'always' ||
      (refetchOnMount && query.isStale.value)
    ) {
      void query.refetch();
    }
  });

  return query;
}
