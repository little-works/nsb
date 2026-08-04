import { Request } from './request';

import type {
  AppLanguage,
  ProfileListResponse,
  ProfileItem,
  RuntimeStatus,
  RuntimeSettings,
  SaveProfilePayload,
  SaveSettingsPayload,
} from '@/types';

const request = new Request();
const kernelDownloadRequest = new Request({ timeout: 300_000 });
const profileActivationRequest = new Request({ timeout: 60_000 });

export interface KernelReleaseInfo {
  version: string;
}

export function fetchRuntime() {
  return request.get<RuntimeStatus>('/api/runtime');
}

export function toggleKernel() {
  return request.post<RuntimeStatus>('/api/kernel/toggle');
}

export function fetchKernelVersion() {
  return request.get<string>('/api/kernel/version');
}

export function fetchLatestKernelRelease() {
  return request.get<KernelReleaseInfo>('/api/kernel/latest');
}

export function downloadLatestKernel() {
  return kernelDownloadRequest.post<KernelReleaseInfo>('/api/kernel/download');
}

export async function importKernelBinary(file: File) {
  const formData = new FormData();
  formData.append('file', file, file.name);
  await request.postForm<string>('/api/kernel/import', formData);
}

export function listProfiles() {
  return request.get<ProfileListResponse>('/api/profiles');
}

export function getProfile(id: string) {
  return request.get<ProfileItem>(`/api/profiles/${id}`);
}

export function createProfile(payload: Omit<SaveProfilePayload, 'id'>) {
  return request.post<ProfileListResponse>('/api/profiles', payload);
}

export function importProfile(fileName: string, content: string) {
  return request.post<ProfileListResponse>('/api/profiles/import', {
    file_name: fileName,
    content,
  });
}

export function updateProfile(
  id: string,
  payload: Omit<SaveProfilePayload, 'id'>,
) {
  return request.put<ProfileListResponse>(`/api/profiles/${id}`, payload);
}

export function deleteProfile(id: string) {
  return request.delete<ProfileListResponse>(`/api/profiles/${id}`);
}

export function getProfileContent(id: string) {
  return request.get<string>(`/api/profiles/${id}/content`);
}

export function saveProfileContent(id: string, content: string) {
  return request.put<string>(`/api/profiles/${id}/content`, { content });
}

export function setCurrentProfile(id: string) {
  return profileActivationRequest.post<ProfileListResponse>(
    `/api/profiles/${id}/activate`,
  );
}

export function refreshProfile() {
  return request.post<ProfileListResponse>('/api/profiles/refresh');
}

export function refreshProfileById(id: string) {
  return request.post<ProfileListResponse>(`/api/profiles/${id}/refresh`);
}

export function saveSettings(payload: SaveSettingsPayload) {
  return request.post<RuntimeSettings>('/api/settings', payload);
}

export function fetchSettings() {
  return request.get<RuntimeSettings>('/api/settings');
}

export function saveAppLanguage(appLanguage: AppLanguage) {
  return request.post<RuntimeSettings>('/api/settings/app-language', {
    app_language: appLanguage,
  });
}

export function fetchAutoLaunchEnabled() {
  return request.get<boolean>('/api/settings/auto-launch');
}

export function updateAutoLaunchEnabled(enabled: boolean) {
  return request.post<boolean>('/api/settings/auto-launch', { enabled });
}
