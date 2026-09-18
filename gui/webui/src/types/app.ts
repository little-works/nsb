import type { AppLanguage } from '@/rs-type/AppLanguage';
import type { ProfileRemoteKeepFields } from '@/rs-type/ProfileRemoteKeepFields';

export type RequestMethod =
  'GET' | 'POST' | 'DELETE' | 'PUT' | 'HEAD' | 'PATCH';
export type RequestProxyMode =
  'global' | 'none' | 'system' | 'kernel' | 'custom';

export type { AppConfig } from '@/rs-type/AppConfig';
export type { AppLanguage } from '@/rs-type/AppLanguage';
export type { ProfileItem } from '@/rs-type/ProfileItem';
export type { ProfileDetailResponse } from '@/rs-type/ProfileDetailResponse';
export type { ProfileHeader } from '@/rs-type/ProfileHeader';
export type { ProfileRemote } from '@/rs-type/ProfileRemote';
export type { ProfileRemoteFormat } from '@/rs-type/ProfileRemoteFormat';
export type { ProfileRemoteKeepFields } from '@/rs-type/ProfileRemoteKeepFields';
export type { ProfileTemplate } from '@/rs-type/ProfileTemplate';
export type { ProfileListResponse } from '@/rs-type/ProfileListResponse';
export type { ProfileSummary } from '@/rs-type/ProfileSummary';
export type { PortableDataArchive } from '@/rs-type/PortableDataArchive';
export type { DataImportReport } from '@/rs-type/DataImportReport';
export type { DataImportIssue } from '@/rs-type/DataImportIssue';
export type { DataImportEntityStats } from '@/rs-type/DataImportEntityStats';

export const AppPageType = {
  Proxies: 'Proxies',
  Profiles: 'Profiles',
  Templates: 'Templates',
  Logs: 'Logs',
  Connections: 'Connections',
  Settings: 'Settings',
} as const;

export type AppPageTypes = (typeof AppPageType)[keyof typeof AppPageType];

export interface KernelInfo {
  binary_path: string;
  installed: boolean;
  data_dir: string;
  config_path: string;
  version: string;
  status: 'Running' | 'Stopped' | 'Failed';
  last_started_at: string;
}

export interface RuntimeStatus {
  kernel: KernelInfo;
}

export interface RuntimeSettings {
  app_port: number;
  system_proxy_enabled: boolean;
  app_language: AppLanguage;
}

export interface ApiResponse<T> {
  ok: boolean;
  message: string;
  data: T | null;
}

export interface SaveProfilePayload {
  name: string;
  template_id: string;
  inline_template?: Record<string, unknown> | null;
  update_interval_hours: number | null;
  update_cron: string | null;
  remotes: Array<{
    name: string;
    url: string;
    headers: Array<{ key: string; value: string }>;
    format: 'clash' | 'singbox';
    keep: ProfileRemoteKeepFields;
  }>;
  hook?: string | null;
}

export interface SaveSettingsPayload {
  app_port: number;
  system_proxy_enabled: boolean;
}

export interface SaveProfileTemplatePayload {
  name: string;
  content: string;
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
