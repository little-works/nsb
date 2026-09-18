import {
  exportPortableData,
  importPortableData,
  previewPortableData,
} from '@/api/client';
import { Button } from '@/components/button';
import { Dialog } from '@/components/dialog';
import { Icon } from '@/components/icon';
import { toast } from '@/components/toast';
import { useClientQuery } from '@/hooks/use-client-query';
import { i18n } from '@/i18n';
import { __render } from '@/shared/helper';
import type { DataImportEntityStats, DataImportReport } from '@/types';
import { useQueryClient } from '@tanstack/vue-query';
import {
  DownloadOutlined,
  ImportExportOutlined,
  UploadFileOutlined,
  WarningAmberOutlined,
} from '@vicons/material';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';

export interface DataManagementSectionProps {}

defineProps<DataManagementSectionProps>();
defineOptions({ name: 'DataManagementSection' });

const fileInput = ref<HTMLInputElement>();
const selectedFile = ref<File | null>(null);
const report = ref<DataImportReport | null>(null);
const dialogOpen = ref(false);
const importComplete = ref(false);
const queryClient = useQueryClient();

function operationError(error: unknown) {
  return error instanceof Error
    ? error.message
    : i18n.global.t('settings.dataOperationFailed');
}

const exportQuery = useClientQuery({
  queryKey: ['settings', 'data', 'export'],
  enabled: false,
  queryFn: async () => {
    try {
      const archive = await exportPortableData();
      const blob = new Blob([JSON.stringify(archive, null, 2)], {
        type: 'application/json',
      });
      const href = URL.createObjectURL(blob);
      const link = document.createElement('a');
      const timestamp = new Date()
        .toISOString()
        .replace(/[-:]/g, '')
        .replace(/\.\d{3}Z$/, 'Z');
      link.href = href;
      link.download = `nsb-backup-${timestamp}.json`;
      link.click();
      URL.revokeObjectURL(href);
      return archive;
    } catch (error) {
      toast.error({
        title: i18n.global.t('settings.dataExportFailed'),
        content: operationError(error),
      });
      throw error;
    }
  },
});

const previewQuery = useClientQuery({
  queryKey: ['settings', 'data', 'import-preview'],
  enabled: false,
  queryFn: async () => {
    const file = selectedFile.value;
    if (!file) throw new Error(i18n.global.t('settings.dataSelectFile'));
    try {
      const nextReport = await previewPortableData(file);
      report.value = nextReport;
      importComplete.value = false;
      dialogOpen.value = true;
      return nextReport;
    } catch (error) {
      selectedFile.value = null;
      toast.error({
        title: i18n.global.t('settings.dataImportFailed'),
        content: operationError(error),
      });
      throw error;
    }
  },
});

const importQuery = useClientQuery({
  queryKey: ['settings', 'data', 'import'],
  enabled: false,
  queryFn: async () => {
    const file = selectedFile.value;
    if (!file) throw new Error(i18n.global.t('settings.dataSelectFile'));
    try {
      const nextReport = await importPortableData(file);
      report.value = nextReport;
      importComplete.value = true;
      selectedFile.value = null;
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['profiles'] }),
        queryClient.invalidateQueries({ queryKey: ['templates'] }),
      ]);
      return nextReport;
    } catch (error) {
      toast.error({
        title: i18n.global.t('settings.dataImportFailed'),
        content: operationError(error),
      });
      throw error;
    }
  },
});

const actionableCount = computed(() => {
  const value = report.value;
  if (!value) return 0;
  return (
    value.templates.added +
    value.templates.overwritten +
    value.profiles.added +
    value.profiles.overwritten
  );
});

function openImportPicker() {
  fileInput.value?.click();
}

function selectImportFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = '';
  if (!file || previewQuery.isFetching.value || importQuery.isFetching.value) {
    return;
  }
  selectedFile.value = file;
  void previewQuery.refetch();
}

function closeDialog() {
  if (importQuery.isFetching.value) return;
  dialogOpen.value = false;
  selectedFile.value = null;
  report.value = null;
  importComplete.value = false;
}

function StatsRow(props: { label: string; value: DataImportEntityStats }) {
  const { t } = useI18n();
  return (
    <div class="grid grid-cols-[minmax(0,1fr)_repeat(4,minmax(3rem,auto))] items-center gap-2 rounded border border-outline-variant bg-surface-container-low px-3 py-2 text-xs">
      <span class="truncate font-medium text-on-surface">{props.label}</span>
      <span class="text-center text-on-surface-variant">
        {t('settings.dataAdded')}: {props.value.added}
      </span>
      <span class="text-center text-on-surface-variant">
        {t('settings.dataOverwritten')}: {props.value.overwritten}
      </span>
      <span class="text-center text-on-surface-variant">
        {t('settings.dataSkipped')}: {props.value.skipped}
      </span>
      <span class="text-center text-on-surface-variant">
        {t('settings.dataWarnings')}: {props.value.warnings}
      </span>
    </div>
  );
}

export default __render<DataManagementSectionProps>(() => {
  const { t } = useI18n();
  const busy =
    exportQuery.isFetching.value ||
    previewQuery.isFetching.value ||
    importQuery.isFetching.value;

  return (
    <div class="mb-10">
      <input
        ref={fileInput}
        accept="application/json,.json"
        class="sr-only"
        type="file"
        onChange={selectImportFile}
      />
      <div class="mb-3 flex items-center gap-2">
        <Icon class="text-xl text-primary">
          <ImportExportOutlined />
        </Icon>
        <h3 class="text-lg font-bold leading-6 text-on-surface">
          {t('settings.dataManagement')}
        </h3>
      </div>
      <div class="rounded border border-outline-variant bg-surface-container-lowest p-4">
        <div class="flex items-start gap-3 rounded border border-warn/40 bg-surface-container-low px-3 py-2 text-on-surface">
          <Icon class="mt-0.5 shrink-0 text-lg text-warn">
            <WarningAmberOutlined />
          </Icon>
          <p class="text-sm leading-5">{t('settings.dataSensitiveWarning')}</p>
        </div>
        <div class="mt-4 flex flex-wrap items-center justify-between gap-4">
          <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-on-surface">
              {t('settings.dataTemplatesProfiles')}
            </p>
            <p class="text-sm leading-5 text-on-surface-variant">
              {t('settings.dataDescription')}
            </p>
          </div>
          <div class="flex items-center gap-2">
            <Button
              disabled={busy}
              shape="rect"
              size="sm"
              variant="outline"
              onClick={openImportPicker}
            >
              <Icon class="text-base">
                <UploadFileOutlined />
              </Icon>
              {t('settings.dataImport')}
            </Button>
            <Button
              disabled={busy}
              shape="rect"
              size="sm"
              variant="solid"
              onClick={() => {
                void exportQuery.refetch();
              }}
            >
              <Icon class="text-base">
                <DownloadOutlined />
              </Icon>
              {t('settings.dataExport')}
            </Button>
          </div>
        </div>
      </div>

      <Dialog
        open={dialogOpen.value}
        title={
          importComplete.value
            ? t('settings.dataImportComplete')
            : t('settings.dataImportPreview')
        }
        description={selectedFile.value?.name ?? ''}
        closeDisabled={importQuery.isFetching.value}
        contentClass="max-w-3xl"
        onClose={closeDialog}
      >
        {report.value ? (
          <div class="space-y-4">
            <div class="space-y-2">
              <StatsRow
                label={t('settings.dataTemplates')}
                value={report.value.templates}
              />
              <StatsRow
                label={t('settings.dataProfiles')}
                value={report.value.profiles}
              />
            </div>
            {report.value.issues.length ? (
              <div class="max-h-64 space-y-2 overflow-y-auto rounded border border-outline-variant p-2">
                {report.value.issues.map((issue, index) => (
                  <div
                    key={`${issue.entity}-${issue.id ?? index}-${index}`}
                    class={[
                      'rounded px-3 py-2',
                      'bg-surface-container-low text-xs text-on-surface',
                    ]}
                  >
                    <p class="font-medium">
                      {issue.level === 'warning'
                        ? t('settings.dataWarning')
                        : t('settings.dataSkippedItem')}{' '}
                      ·{' '}
                      {issue.entity === 'template'
                        ? t('settings.dataTemplate')
                        : t('settings.dataProfile')}{' '}
                      {issue.name || issue.id || ''}
                    </p>
                    <p class="mt-1 leading-5 text-on-surface-variant">
                      {issue.reason}
                    </p>
                  </div>
                ))}
              </div>
            ) : (
              <p class="text-sm text-on-surface-variant">
                {t('settings.dataNoIssues')}
              </p>
            )}
            <div class="flex items-center justify-end gap-2 border-t border-outline-variant pt-4">
              <Button
                disabled={importQuery.isFetching.value}
                onClick={closeDialog}
              >
                {importComplete.value ? t('common.close') : t('common.cancel')}
              </Button>
              {!importComplete.value ? (
                <Button
                  disabled={
                    actionableCount.value === 0 || importQuery.isFetching.value
                  }
                  variant="solid"
                  onClick={() => {
                    void importQuery.refetch();
                  }}
                >
                  {t('settings.dataConfirmImport')}
                </Button>
              ) : null}
            </div>
          </div>
        ) : null}
      </Dialog>
    </div>
  );
});
