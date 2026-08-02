import {
  fetchAutoLaunchEnabled,
  fetchKernelVersion,
  fetchLatestKernelRelease,
  saveSettings,
  updateAutoLaunchEnabled,
} from '@/api/client';
import { toast } from '@/components/toast';
import { useClientQuery } from '@/hooks/use-client-query';
import { useAppSnapshot } from '@/store/app';
import type { AppSnapshot } from '@/types';
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { i18n } from '@/i18n';

export const useSettingsStore = defineStore('settings', () => {
  const appSnapshot = useAppSnapshot();
  const saving = ref(false);
  const mixedPort = ref('7990');
  const allowLan = ref(false);
  const systemProxyEnabled = ref(false);
  const requestedAutoLaunchEnabled = ref(false);
  const kernelVersion = ref('--');
  const latestKernelVersion = ref('');
  const initialMixedPort = ref('7990');
  const initialAllowLan = ref(false);
  const initialSystemProxyEnabled = ref(false);

  const settingsDirty = computed(
    () =>
      mixedPort.value !== initialMixedPort.value ||
      allowLan.value !== initialAllowLan.value ||
      systemProxyEnabled.value !== initialSystemProxyEnabled.value,
  );

  function syncFormWithSnapshot(snapshot: AppSnapshot) {
    mixedPort.value = String(snapshot.state.gui_config.mixed_port);
    allowLan.value = snapshot.state.gui_config.allow_lan;
    systemProxyEnabled.value = snapshot.state.gui_config.system_proxy_enabled;
    initialMixedPort.value = mixedPort.value;
    initialAllowLan.value = allowLan.value;
    initialSystemProxyEnabled.value = systemProxyEnabled.value;
    kernelVersion.value = snapshot.state.kernel.version;
  }

  const settingsQuery = useClientQuery({
    queryKey: ['settings'],
    staleTime: 30 * 1000,
    queryFn: async () => {
      try {
        const result = await appSnapshot.refetch();
        if (!result.data) {
          throw new Error(i18n.global.t('errors.loadSettings'));
        }
        syncFormWithSnapshot(result.data);
        const [localVersion, release] = await Promise.allSettled([
          fetchKernelVersion(),
          fetchLatestKernelRelease(),
        ]);
        if (localVersion.status === 'fulfilled') {
          kernelVersion.value = localVersion.value;
        } else {
          kernelVersion.value = '--';
        }
        if (release.status === 'fulfilled') {
          latestKernelVersion.value = release.value.version;
        } else {
          toast.error({
            content:
              release.reason instanceof Error
                ? release.reason.message
                : i18n.global.t('errors.kernelDownload'),
            title: i18n.global.t('errors.kernelAction'),
          });
        }
        return result.data;
      } catch (error) {
        toast.error({
          content:
            error instanceof Error
              ? error.message
              : i18n.global.t('errors.loadSettings'),
          title: i18n.global.t('errors.loadSettings'),
        });
        throw error;
      }
    },
  });

  const autoLaunchQuery = useClientQuery({
    queryKey: ['autoLaunch'],
    staleTime: 30 * 1000,
    queryFn: async () => {
      try {
        return await fetchAutoLaunchEnabled();
      } catch (error) {
        toast.error({
          content:
            error instanceof Error
              ? error.message
              : i18n.global.t('errors.autoLaunchRead'),
          title: i18n.global.t('errors.autoLaunchRead'),
        });
        throw error;
      }
    },
  });

  const autoLaunchUpdateQuery = useClientQuery({
    queryKey: ['autoLaunch', 'update'],
    enabled: false,
    queryFn: async () => {
      try {
        await updateAutoLaunchEnabled(requestedAutoLaunchEnabled.value);
      } catch (error) {
        toast.error({
          content:
            error instanceof Error
              ? error.message
              : i18n.global.t('errors.autoLaunchUpdate'),
          title: i18n.global.t('errors.autoLaunchUpdate'),
        });
        throw error;
      }
    },
  });

  const loading = computed(() => settingsQuery.isFetching.value);
  const autoLaunchEnabled = computed(() => autoLaunchQuery.data.value ?? false);
  const autoLaunchLoading = computed(
    () =>
      autoLaunchQuery.isFetching.value ||
      autoLaunchUpdateQuery.isFetching.value,
  );

  function loadSettings() {
    void settingsQuery.refetch();
    void autoLaunchQuery.refetch();
  }

  async function setAutoLaunchEnabled(enabled: boolean) {
    if (autoLaunchUpdateQuery.isFetching.value) {
      return;
    }

    requestedAutoLaunchEnabled.value = enabled;
    try {
      await autoLaunchUpdateQuery.refetch();
    } finally {
      // An OS registration may change before reporting an error. Always use a
      // fresh system read as the authoritative checkbox state.
      await autoLaunchQuery.refetch();
    }
  }

  async function saveRuntimeSettings() {
    if (saving.value) {
      return;
    }

    const parsedPort = Number.parseInt(mixedPort.value, 10);
    if (
      !Number.isInteger(parsedPort) ||
      parsedPort <= 0 ||
      parsedPort > 65535
    ) {
      toast.error({ title: i18n.global.t('errors.invalidPort') });
      return false;
    }

    saving.value = true;
    try {
      await saveSettings({
        mixed_port: parsedPort,
        allow_lan: allowLan.value,
        system_proxy_enabled: systemProxyEnabled.value,
      });
      const result = await appSnapshot.refetch();
      if (!result.data) {
        throw new Error(i18n.global.t('errors.loadSettings'));
      }
      syncFormWithSnapshot(result.data);
      toast.info({ title: i18n.global.t('errors.settingsSaved') });
      return true;
    } catch (error) {
      toast.error({
        content:
          error instanceof Error
            ? error.message
            : i18n.global.t('errors.saveSettings'),
        title: i18n.global.t('errors.saveSettingsTitle'),
      });
      return false;
    } finally {
      saving.value = false;
    }
  }

  function setMixedPort(value: string) {
    mixedPort.value = value;
  }

  async function saveIfDirty() {
    if (settingsDirty.value) {
      return saveRuntimeSettings();
    }
    return false;
  }

  async function updateAllowLan(value: boolean) {
    allowLan.value = value;
    return saveIfDirty();
  }

  async function updateSystemProxyEnabled(value: boolean) {
    systemProxyEnabled.value = value;
    return saveIfDirty();
  }

  return {
    allowLan,
    autoLaunchEnabled,
    autoLaunchLoading,
    kernelVersion,
    latestKernelVersion,
    loading,
    mixedPort,
    saving,
    settingsDirty,
    systemProxyEnabled,
    loadSettings,
    saveIfDirty,
    saveRuntimeSettings,
    setAutoLaunchEnabled,
    setMixedPort,
    updateAllowLan,
    updateSystemProxyEnabled,
  };
});
