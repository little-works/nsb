import { __render } from '@/shared/helpter';
import { useScoreStreamConnection } from '@/hooks/use-score-stream';
import { useAppPageType } from '@/store/app';
import { useSlots } from 'vue';
import { useI18n } from 'vue-i18n';
import AppShell from './app-shell/index.setup';
import { type NavItem } from '@/pages/index/types';
import { AppPageType } from '@/types';
import {
  HomeFilled,
  HomeOutlined,
  HubOutlined,
  LayersOutlined,
  SettingsFilled,
  SettingsOutlined,
  SyncFilled,
  SyncOutlined,
  TerminalOutlined,
} from '@vicons/material';

export interface NsbLayoutProps {}

defineProps<NsbLayoutProps>();

const slots = useSlots();
const pageType = useAppPageType();
useScoreStreamConnection();

const navItems: NavItem[] = [
  {
    key: AppPageType.Proxies,
    href: '/webui/',
    label: 'Proxies',
    caption: 'Proxy Manager',
    icon: HomeOutlined,
    activeIcon: HomeFilled,
  },
  {
    key: AppPageType.Profiles,
    href: '/webui/profiles',
    label: 'Profiles',
    caption: 'Profile Management',
    icon: SyncOutlined,
    activeIcon: SyncFilled,
  },
  {
    key: AppPageType.Templates,
    href: '/webui/templates',
    label: 'Templates',
    caption: 'Template Management',
    icon: LayersOutlined,
    activeIcon: LayersOutlined,
  },
  {
    key: AppPageType.Logs,
    href: '/webui/logs',
    label: 'Logs',
    caption: 'Kernel Logs',
    icon: TerminalOutlined,
    activeIcon: TerminalOutlined,
  },
  {
    key: AppPageType.Connections,
    href: '/webui/connections',
    label: 'Connections',
    caption: 'Kernel Connections',
    icon: HubOutlined,
    activeIcon: HubOutlined,
  },
  {
    key: AppPageType.Settings,
    href: '/webui/settings',
    label: 'Settings',
    caption: 'Settings',
    icon: SettingsOutlined,
    activeIcon: SettingsFilled,
  },
];

defineOptions({ name: 'NsbLayout' });
const { t } = useI18n();

export default __render<NsbLayoutProps>(() => {
  const translatedNavItems = navItems.map((item) => ({
    ...item,
    label: t(`nav.${item.key.toLowerCase()}`),
    caption: t(
      `nav.${item.key.toLowerCase()}${item.key === AppPageType.Proxies || item.key === AppPageType.Profiles || item.key === AppPageType.Templates || item.key === AppPageType.Logs || item.key === AppPageType.Connections ? 'Caption' : ''}`,
    ),
  }));
  return (
    <AppShell navItems={translatedNavItems} selectedView={pageType.value}>
      {slots.default?.()}
    </AppShell>
  );
});
