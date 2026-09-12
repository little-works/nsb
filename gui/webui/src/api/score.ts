import { Request } from './request';

import type {
  CoreApiConfig,
  CoreApiConnectionsData,
  CoreApiProxies,
} from '@/types';

const request = new Request();

export function getScoreProxies() {
  return request.get<CoreApiProxies>('/api/score/proxies');
}

export function getScoreConfig() {
  return request.get<CoreApiConfig>('/api/score/configs');
}

export function getScoreConnections() {
  return request.get<CoreApiConnectionsData>('/api/score/connections');
}

export async function setScoreMode(mode: 'global' | 'rule' | 'direct') {
  await request.patch<boolean>('/api/score/configs', { mode });
}

export async function selectScoreProxy(group: string, name: string) {
  await request.put<boolean>(`/api/score/proxies/${group}`, { name });
}

export function startScoreLatencyTest(proxies: string[]) {
  return request.post<boolean>('/api/score/latency', { proxies });
}

export function getScoreLogHistory() {
  return request.get<string[]>('/api/score/logs/history');
}

export function clearScoreLogs() {
  return request.post<boolean>('/api/score/logs');
}
