import {
  fetchAutoLaunchEnabled,
  fetchKernelVersion,
  fetchLatestKernelRelease,
  saveSettings,
  updateAutoLaunchEnabled,
} from '@/api/client';
import { toast } from '@/components/toast';
import { useClientQuery } from '@/hooks/use-client-query';
import { useRuntimeSettings, useRuntimeStatus } from '@/store/app';
import type { RuntimeSettings } from '@/types';
import { defineStore } from 'pinia';
import { computed, ref, watch } from 'vue';
import { i18n } from '@/i18n';

export const useSettingsStore = defineStore('settings', () => {
  const runtimeSettings = useRuntimeSettings();
  const runtimeStatus = useRuntimeStatus();
  const saving = ref(false);
  const appPort = ref('8787');
  const systemProxyEnabled = ref(false);
  const requestedAutoLaunchEnabled = ref(false);
  const kernelVersion = ref('--');
  const latestKernelVersion = ref('');
  const initialAppPort = ref('8787');
  const initialSystemProxyEnabled = ref(false);

  const settingsDirty = computed(
    () =>
      appPort.value !== initialAppPort.value ||
      systemProxyEnabled.value !== initialSystemProxyEnabled.value,
  );

  function syncFormWithSettings(settings: RuntimeSettings) {
    appPort.value = String(settings.app_port);
    systemProxyEnabled.value = settings.system_proxy_enabled;
    initialAppPort.value = appPort.value;
    initialSystemProxyEnabled.value = systemProxyEnabled.value;
  }

  watch(
    runtimeSettings.data,
    (settings) => {
      if (settings) {
        syncFormWithSettings(settings);
      }
    },
    { immediate: true },
  );

  watch(runtimeSettings.error, (error) => {
    if (!error) {
      return;
    }

    toast.error({
      content:
        error instanceof Error
          ? error.message
          : i18n.global.t('errors.loadSettings'),
      title: i18n.global.t('errors.loadSettings'),
    });
  });

  const kernelInfoQuery = useClientQuery({
    queryKey: ['kernelInfo'],
    staleTime: 30 * 1000,
    queryFn: async () => {
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
      return {
        kernelVersion: kernelVersion.value,
        latestKernelVersion: latestKernelVersion.value,
      };
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

  const loading = computed(
    () => runtimeSettings.isFetching.value || kernelInfoQuery.isFetching.value,
  );
  const autoLaunchEnabled = computed(() => autoLaunchQuery.data.value ?? false);
  const autoLaunchLoading = computed(
    () =>
      autoLaunchQuery.isFetching.value ||
      autoLaunchUpdateQuery.isFetching.value,
  );

  function loadSettings() {
    void runtimeSettings.refetch();
    void kernelInfoQuery.refetch();
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

    const parsedAppPort = Number.parseInt(appPort.value, 10);
    if (
      !Number.isInteger(parsedAppPort) ||
      parsedAppPort <= 0 ||
      parsedAppPort > 65535
    ) {
      toast.error({ title: i18n.global.t('errors.invalidAppPort') });
      return false;
    }

    const appPortChanged = appPort.value !== initialAppPort.value;
    saving.value = true;
    try {
      await saveSettings({
        app_port: parsedAppPort,
        system_proxy_enabled: systemProxyEnabled.value,
      });
      const result = await runtimeSettings.refetch();
      if (!result.data) {
        throw new Error(i18n.global.t('errors.loadSettings'));
      }
      syncFormWithSettings(result.data);
      void runtimeStatus.refetch();
      toast.info({
        title: i18n.global.t('errors.settingsSaved'),
        content: appPortChanged
          ? i18n.global.t('settings.appPortRestartRequired')
          : undefined,
      });
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

  function setAppPort(value: string) {
    appPort.value = value;
  }

  async function saveIfDirty() {
    if (settingsDirty.value) {
      return saveRuntimeSettings();
    }
    return false;
  }

  async function updateSystemProxyEnabled(value: boolean) {
    systemProxyEnabled.value = value;
    return saveIfDirty();
  }

  return {
    appPort,
    autoLaunchEnabled,
    autoLaunchLoading,
    kernelVersion,
    latestKernelVersion,
    loading,
    saving,
    settingsDirty,
    systemProxyEnabled,
    loadSettings,
    saveIfDirty,
    saveRuntimeSettings,
    setAutoLaunchEnabled,
    setAppPort,
    updateSystemProxyEnabled,
  };
});
