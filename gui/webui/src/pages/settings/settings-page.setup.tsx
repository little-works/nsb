import AppearanceSection from '@/pages/settings/appearance-section.setup';
import GeneralSection from '@/pages/settings/general-section.setup';
import KernelInfoSection from '@/pages/settings/kernel-info-section.setup';
import { __render } from '@/shared/helper';
import { IconButton } from '@/components/button';
import { Page, PageContent } from '@/components/page-content';
import { useSettingsStore } from '@/store/settings';
import { RefreshOutlined } from '@vicons/material';
import { storeToRefs } from 'pinia';

export interface SettingsPageProps {}

defineProps<SettingsPageProps>();

const settingsStore = useSettingsStore();
const {
  appPort,
  autoLaunchEnabled,
  autoLaunchLoading,
  kernelVersion,
  latestKernelVersion,
  loading,
  saving,
  systemProxyEnabled,
} = storeToRefs(settingsStore);

defineOptions({ name: 'SettingsPage' });

export default __render<SettingsPageProps>(() => (
  <Page class="min-h-screen bg-background" title="Settings" subtitle="subtitle">
    {{
      actions: () => (
        <IconButton
          disabled={loading.value}
          onClick={settingsStore.loadSettings}
        >
          <RefreshOutlined />
        </IconButton>
      ),
      default: () => (
        <PageContent class="pb-24 pt-4">
          <GeneralSection
            appPort={appPort.value}
            autoLaunchEnabled={autoLaunchEnabled.value}
            autoLaunchLoading={autoLaunchLoading.value}
            loading={loading.value}
            saving={saving.value}
            systemProxyEnabled={systemProxyEnabled.value}
            onAppPortChange={settingsStore.setAppPort}
            onAutoLaunchEnabledChange={async (value) => {
              await settingsStore.setAutoLaunchEnabled(value);
            }}
            onAppPortBlur={async () => {
              await settingsStore.saveIfDirty();
            }}
            onSystemProxyEnabledChange={async (value) => {
              await settingsStore.updateSystemProxyEnabled(value);
            }}
          />
          <KernelInfoSection
            latestVersion={latestKernelVersion.value}
            version={kernelVersion.value}
            onReload={settingsStore.loadSettings}
          />
          <AppearanceSection />
        </PageContent>
      ),
    }}
  </Page>
));
