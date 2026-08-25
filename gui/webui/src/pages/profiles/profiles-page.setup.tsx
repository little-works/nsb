import {
  deleteProfile,
  importProfile,
  refreshProfileById,
  setCurrentProfile,
} from '@/api/client';
import { IconButton } from '@/components/button';
import { Page, PageContent } from '@/components/page-content';
import { toast } from '@/components/toast';
import ProfilesTable from '@/pages/profiles/profiles-table.setup';
import { __render } from '@/shared/helpter';
import { useProfiles, useRuntimeStatus } from '@/store/app';
import { AddOutlined, FileUploadOutlined } from '@vicons/material';
import { fileOpen } from 'browser-fs-access';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { navigate } from 'vike/client/router';
import { i18n } from '@/i18n';

export interface ProfilesPageProps {}

defineProps<ProfilesPageProps>();

const profilesQuery = useProfiles();
const runtimeStatus = useRuntimeStatus();
let switchingProfile = false;
const loading = computed(() => profilesQuery.isFetching.value);
const profiles = computed(() => profilesQuery.data.value?.profiles ?? []);
const currentProfileId = computed(
  () => profilesQuery.data.value?.current_profile_id ?? null,
);
const currentProfile = computed(
  () =>
    profiles.value.find((item) => item.id === currentProfileId.value) ?? null,
);

function openCreatePage() {
  void navigate('/webui/profiles/edit');
}

async function refreshProfiles() {
  const result = await profilesQuery.refetch();
  if (!result.data) throw new Error(i18n.global.t('profiles.loadFailed'));
  return result.data;
}

async function handleImportProfile() {
  try {
    const file = await fileOpen({
      extensions: ['.json'],
      mimeTypes: ['application/json', 'text/json'],
    });
    await importProfile(file.name, await file.text());
    await refreshProfiles();
    toast.info({ title: i18n.global.t('profiles.imported') });
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') return;
    toast.error({
      title:
        error instanceof Error
          ? error.message
          : i18n.global.t('profiles.importFailed'),
    });
  }
}

function startEdit(id: string) {
  void navigate(`/webui/profiles/edit?id=${encodeURIComponent(id)}`);
}

async function handleDelete(id: string, name: string) {
  if (!window.confirm(i18n.global.t('profiles.confirmDelete', { name })))
    return;
  try {
    await deleteProfile(id);
    await refreshProfiles();
    toast.info({ title: i18n.global.t('profiles.deleted', { name }) });
  } catch (error) {
    toast.error({
      content:
        error instanceof Error
          ? error.message
          : i18n.global.t('profiles.deleteFailed'),
      title: i18n.global.t('profiles.operationFailed'),
    });
  }
}

async function handleSetCurrent(id: string) {
  if (id === currentProfileId.value || switchingProfile) return;
  try {
    switchingProfile = true;
    await setCurrentProfile(id);
    await refreshProfiles();
    await runtimeStatus.refetch();
  } catch (error) {
    toast.error({
      content:
        error instanceof Error
          ? error.message
          : i18n.global.t('profiles.activateFailed'),
      title: i18n.global.t('profiles.operationFailed'),
    });
  } finally {
    switchingProfile = false;
  }
}

async function handleRefresh(id: string, name: string) {
  try {
    await refreshProfileById(id);
    await refreshProfiles();
    await runtimeStatus.refetch();
    toast.info({ title: i18n.global.t('profiles.refreshed', { name }) });
  } catch (error) {
    toast.error({
      content:
        error instanceof Error
          ? error.message
          : i18n.global.t('profiles.refreshFailed'),
      title: i18n.global.t('profiles.operationFailed'),
    });
  }
}

defineOptions({ name: 'ProfilesPage' });
const { t } = useI18n();

export default __render<ProfilesPageProps>(() => (
  <Page
    title={t('profiles.title')}
    subtitle={t('profiles.current', {
      name: currentProfile.value?.name ?? t('profiles.noneSelected'),
    })}
  >
    {{
      actions: () => (
        <>
          <IconButton
            aria-label={t('profiles.add')}
            size="sm"
            tooltip={t('profiles.add')}
            onClick={openCreatePage}
          >
            <AddOutlined />
          </IconButton>
          <IconButton
            aria-label={t('profiles.import')}
            size="sm"
            tooltip={t('profiles.import')}
            onClick={() => void handleImportProfile()}
          >
            <FileUploadOutlined />
          </IconButton>
        </>
      ),
      default: () => (
        <PageContent class="space-y-4 pb-24 pt-4">
          <ProfilesTable
            profiles={profiles.value}
            currentProfileId={currentProfileId.value}
            loading={loading.value}
            onEdit={(item) => startEdit(item.id)}
            onDelete={(item) => void handleDelete(item.id, item.name)}
            onRefresh={(item) => void handleRefresh(item.id, item.name)}
            onSetCurrent={(item) => void handleSetCurrent(item.id)}
          />
        </PageContent>
      ),
    }}
  </Page>
));
