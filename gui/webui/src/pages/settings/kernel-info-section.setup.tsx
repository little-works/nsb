import { __render } from '@/shared/helpter';
import {
  downloadLatestKernel,
  fetchKernelDownloadProgress,
  importKernelBinary,
} from '@/api/client';
import { Button } from '@/components/button';
import { Icon } from '@/components/icon';
import { toast } from '@/components/toast';
import {
  DownloadOutlined,
  InfoOutlined,
  UploadFileOutlined,
} from '@vicons/material';
import { useClientQuery } from '@/hooks/use-client-query';
import { useI18n } from 'vue-i18n';
import { computed, ref } from 'vue';
import { i18n } from '@/i18n';

export interface KernelInfoSectionProps {
  version: string;
  latestVersion: string;
  onReload?: () => void | Promise<void>;
}

const props = defineProps<KernelInfoSectionProps>();
const fileInput = ref<HTMLInputElement>();
const importing = ref(false);
const downloadQuery = useClientQuery({
  queryKey: ['downloadLatestKernel'],
  queryFn: downloadLatestKernel,
  enabled: false,
});
const downloadProgressQuery = useClientQuery({
  queryKey: ['kernelDownloadProgress'],
  queryFn: fetchKernelDownloadProgress,
  refetchInterval: (query) => (query.state.data?.in_progress ? 500 : false),
});
const downloading = computed(
  () => downloadProgressQuery.data.value?.in_progress ?? false,
);
const downloadPercent = computed(() => {
  const progress = downloadProgressQuery.data.value;
  if (!progress?.total || progress.total <= 0) {
    return null;
  }
  return Math.min(
    100,
    Math.floor((progress.downloaded / progress.total) * 100),
  );
});

function openImportPicker() {
  fileInput.value?.click();
}

function displayVersion(version: string) {
  return version.replace(/^sing-box version\s+|^v/i, '').trim() || '--';
}

async function importKernel(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = '';
  if (!file || importing.value) {
    return;
  }

  importing.value = true;
  try {
    await importKernelBinary(file);
    await props.onReload?.();
    toast.info({ title: i18n.global.t('errors.kernelImported') });
  } catch (error) {
    toast.error({
      content:
        error instanceof Error
          ? error.message
          : i18n.global.t('errors.kernelImport'),
      title: i18n.global.t('errors.kernelAction'),
    });
  } finally {
    importing.value = false;
  }
}

async function downloadKernel() {
  if (!window.confirm(i18n.global.t('errors.confirmKernelDownload'))) {
    return;
  }

  try {
    const result = await downloadQuery.refetch();
    if (result.error) {
      throw result.error;
    }
    if (!result.data) {
      throw new Error(i18n.global.t('errors.kernelDownload'));
    }
    void downloadProgressQuery.refetch();
  } catch (error) {
    toast.error({
      content:
        error instanceof Error
          ? error.message
          : i18n.global.t('errors.kernelDownload'),
      title: i18n.global.t('errors.kernelAction'),
    });
  }
}

defineOptions({ name: 'KernelInfoSection' });

export default __render<KernelInfoSectionProps>(() => <KernelInfoContent />);

function KernelInfoContent() {
  const { t } = useI18n();
  return (
    <div class="mb-10">
      <input
        ref={fileInput}
        class="sr-only"
        type="file"
        onChange={(event) => {
          void importKernel(event);
        }}
      />
      <div class="mb-3 flex items-center gap-2">
        <Icon class="text-xl text-primary">
          <InfoOutlined />
        </Icon>
        <h3 class="text-lg font-bold leading-6 text-foreground">
          {t('settings.kernelInfo')}
        </h3>
      </div>
      <div class="flex min-h-20 flex-wrap items-center gap-4 rounded border border-outline-variant bg-surface-container-lowest px-4 py-3">
        <div class="grid w-full grid-cols-1 gap-2 sm:w-80 sm:grid-cols-2">
          <div class="min-w-0 rounded border border-outline-variant bg-surface-container-low px-3 py-2">
            <p class="mb-1 text-xs font-medium uppercase tracking-wide text-on-surface-variant">
              {t('settings.localVersion')}
            </p>
            <p class="truncate font-mono text-sm font-semibold text-on-surface">
              {displayVersion(props.version)}
            </p>
          </div>
          <div class="min-w-0 rounded border border-primary/40 px-3 py-2">
            <p class="mb-1 text-xs font-medium uppercase tracking-wide text-primary">
              {t('settings.latestRelease')}
            </p>
            <p class="truncate font-mono text-sm font-semibold text-on-surface">
              {displayVersion(props.latestVersion)}
            </p>
          </div>
        </div>
        <div class="ml-auto flex items-center gap-2">
          <Button
            disabled={importing.value}
            shape="rect"
            size="sm"
            variant="outline"
            onClick={openImportPicker}
          >
            <Icon class="text-base">
              <UploadFileOutlined />
            </Icon>
            {t('settings.import')}
          </Button>
          <Button
            disabled={downloading.value}
            shape="rect"
            size="sm"
            variant="solid"
            onClick={downloadKernel}
          >
            <Icon class="text-base">
              <DownloadOutlined />
            </Icon>
            {downloading.value && downloadPercent.value != null
              ? `${downloadPercent.value}%`
              : t('settings.download')}
          </Button>
        </div>
      </div>
    </div>
  );
}
