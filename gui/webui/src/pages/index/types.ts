import { type AppPageTypes } from '@/types';
import type { Component } from 'vue';

export interface NavItem {
  key: AppPageTypes;
  href: string;
  label: string;
  caption: string;
  icon: Component;
  activeIcon: Component;
}

export interface SummaryStat {
  label: string;
  value: string;
  detail: string;
}

export interface ProxyItem {
  name: string;
  latency: string;
  latencyMs?: number;
  active?: boolean;
  tag?: string;
  alive?: boolean;
}

export type ProxySortMode = 'none' | 'asc' | 'desc';

export type ProxyMode = 'global' | 'rule' | 'direct';

export interface ProxyGroup {
  title: string;
  type: string;
  groupType: string;
  active: string;
  items: ProxyItem[];
}

export interface SettingGroup {
  title: string;
  detail: string;
  items: Array<{
    label: string;
    value: string;
  }>;
}
