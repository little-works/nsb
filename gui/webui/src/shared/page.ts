import { AppPageType, type AppPageTypes } from '@/types';

export function normalizeWebuiPathname(pathname?: string | null) {
  const rawPath = pathname || '/';

  if (rawPath === '/webui') {
    return '/';
  }

  if (rawPath.startsWith('/webui/')) {
    return rawPath.slice('/webui'.length) || '/';
  }

  return rawPath;
}

export function resolveAppPageType(pathname?: string | null): AppPageTypes {
  const normalizedPath = normalizeWebuiPathname(pathname);

  if (
    normalizedPath === '/profiles' ||
    normalizedPath.startsWith('/profiles/')
  ) {
    return AppPageType.Profiles;
  }

  if (normalizedPath === '/logs') {
    return AppPageType.Logs;
  }

  if (normalizedPath === '/connections') {
    return AppPageType.Connections;
  }

  if (normalizedPath === '/settings') {
    return AppPageType.Settings;
  }

  return AppPageType.Proxies;
}
