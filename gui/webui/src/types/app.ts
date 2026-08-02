export type RequestMethod =
  'GET' | 'POST' | 'DELETE' | 'PUT' | 'HEAD' | 'PATCH';
export type RequestProxyMode =
  'global' | 'none' | 'system' | 'kernel' | 'custom';

export type { AppConfig } from '@/rs-type/AppConfig';
export type { AppLanguage } from '@/rs-type/AppLanguage';
export type { ProfileItem } from '@/rs-type/ProfileItem';
export type { ProfileHeader } from '@/rs-type/ProfileHeader';
export type { ProfileRemote } from '@/rs-type/ProfileRemote';
export type { ProfileKind } from '@/rs-type/ProfileKind';
export type { ProfileListResponse } from '@/rs-type/ProfileListResponse';

import type { AppConfig as RsAppConfig } from '@/rs-type/AppConfig';

export const AppPageType = {
  Proxies: 'Proxies',
  Profiles: 'Profiles',
  Logs: 'Logs',
  Connections: 'Connections',
  Settings: 'Settings',
} as const;

export type AppPageTypes = (typeof AppPageType)[keyof typeof AppPageType];

export interface KernelInfo {
  binary_path: string;
  data_dir: string;
  config_path: string;
  version: string;
  status: 'Running' | 'Stopped';
  last_started_at: string;
}

export interface AppState {
  kernel: KernelInfo;
  gui_config: RsAppConfig;
}

export interface AppSnapshot {
  state: AppState;
  kernel_running: boolean;
}

export interface ApiResponse<T> {
  ok: boolean;
  message: string;
  data: T | null;
}

export interface SaveProfilePayload {
  id: string | null;
  name: string;
  source: string;
  content?: string | null;
  headers: Array<{ key: string; value: string }>;
  update_interval_hours: number | null;
  update_cron: string | null;
  remotes?: Array<{
    name: string;
    url: string;
    headers: Array<{ key: string; value: string }>;
  }>;
  hook?: string | null;
}

export interface SaveSettingsPayload {
  mixed_port: number;
  allow_lan: boolean;
  system_proxy_enabled: boolean;
}

export interface AppSettings {
  requestProxyMode: RequestProxyMode;
  customProxy: string;
  closeKernelOnExit: boolean;
  systemProxyServices: string[];
  proxyBypassList: string;
  systemProxyDNS: string;
  systemDefaultDNS: string;
  userAgent: string;
  githubApiToken: string;
  githubDownloadAcceleration: boolean;
  githubDownloadMirror: string;
  pages: string[];
}
