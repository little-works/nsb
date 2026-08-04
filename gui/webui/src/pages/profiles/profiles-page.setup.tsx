import {
  createProfile,
  deleteProfile,
  getProfileContent,
  importProfile,
  refreshProfileById,
  saveProfileContent,
  setCurrentProfile,
  updateProfile,
} from '@/api/client';
import { IconButton } from '@/components/button';
import { useClientQuery } from '@/hooks/use-client-query';
import { Page, PageContent } from '@/components/page-content';
import ProfileDialog from '@/pages/profiles/profile-dialog.setup';
import ProfileNodeEditor from '@/pages/profiles/profile-node-editor.setup';
import ProfilesTable from '@/pages/profiles/profiles-table.setup';
import { toast } from '@/components/toast';
import { __render } from '@/shared/helpter';
import { useProfiles, useRuntimeStatus } from '@/store/app';
import type { ProfileRemote, ProfileSummary } from '@/types';
import { computed, ref } from 'vue';
import { AddOutlined, FileUploadOutlined } from '@vicons/material';
import { fileOpen } from 'browser-fs-access';
import { navigate } from 'vike/client/router';
import { useI18n } from 'vue-i18n';
import { i18n } from '@/i18n';

export interface ProfilesPageProps {}

defineProps<ProfilesPageProps>();

const profilesQuery = useProfiles();
const runtimeStatus = useRuntimeStatus();
const saving = ref(false);
const editingId = ref<string | null>(null);
const dialogOpen = ref(false);
const formName = ref('');
const formSource = ref('');
const formHeaders = ref<import('@/types').ProfileHeader[]>([]);
const formRemotes = ref<ProfileRemote[]>([]);
const formHook = ref('');
const formKeepSubscriptionGroupsAndRules = ref(false);
const updateIntervalHours = ref('');
const updateCron = ref('');
const nodeEditorOpen = ref(false);
const nodeEditorProfile = ref<ProfileSummary | null>(null);
const nodeEditorContent = ref('');
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
const profileContentQuery = useClientQuery({
  queryKey: ['profileContent'],
  queryFn: async () => {
    const profile = nodeEditorProfile.value;
    if (!profile) {
      throw new Error(i18n.global.t('profiles.noneSelected'));
    }
    const content = await getProfileContent(profile.id);
    nodeEditorContent.value = content;
    return content;
  },
  enabled: false,
});

const saveProfileContentQuery = useClientQuery({
  queryKey: ['saveProfileContent'],
  queryFn: async () => {
    const profile = nodeEditorProfile.value;
    if (!profile) {
      throw new Error(i18n.global.t('profiles.noneSelected'));
    }
    await saveProfileContent(profile.id, nodeEditorContent.value);
    await refreshProfiles();
    return true;
  },
  enabled: false,
});

function resetForm() {
  dialogOpen.value = false;
  editingId.value = null;
  formName.value = '';
  formSource.value = '';
  formHeaders.value = [];
  formRemotes.value = [];
  formHook.value = '';
  formKeepSubscriptionGroupsAndRules.value = false;
  updateIntervalHours.value = '';
  updateCron.value = '';
}

function openCreateDialog() {
  void navigate('/webui/profiles/edit');
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

function closeNodeEditor() {
  nodeEditorOpen.value = false;
  nodeEditorProfile.value = null;
  nodeEditorContent.value = '';
}

function openNodeEditor(item: ProfileSummary) {
  nodeEditorProfile.value = item;
  nodeEditorContent.value = '';
  nodeEditorOpen.value = true;
  void profileContentQuery.refetch().then((result) => {
    if (result.error) {
      toast.error({
        title:
          result.error instanceof Error
            ? result.error.message
            : i18n.global.t('profiles.readNodesFailed'),
      });
      closeNodeEditor();
    }
  });
}

function saveNodeEditorContent() {
  void saveProfileContentQuery.refetch().then((result) => {
    if (result.error) {
      toast.error({
        title:
          result.error instanceof Error
            ? result.error.message
            : i18n.global.t('profiles.saveNodesFailed'),
      });
      return;
    }
    toast.info({ title: i18n.global.t('profiles.nodesSaved') });
    closeNodeEditor();
  });
}

function startEdit(item: ProfileSummary) {
  void navigate(`/webui/profiles/edit?id=${encodeURIComponent(item.id)}`);
}

async function refreshProfiles() {
  const result = await profilesQuery.refetch();
  if (!result.data) {
    throw new Error(i18n.global.t('profiles.loadFailed'));
  }
  return result.data;
}

function ensureProfiles(
  nextSnapshot: import('@/types').ProfileListResponse | null | undefined,
) {
  if (!nextSnapshot) {
    throw new Error(i18n.global.t('profiles.refreshFailed'));
  }
  return nextSnapshot;
}

async function handleSubmit() {
  saving.value = true;
  try {
    const payload = {
      name: formName.value,
      source: formRemotes.value[0]?.url.trim() ?? formSource.value.trim(),
      content: null,
      headers:
        formRemotes.value[0]?.headers.filter((header) => header.key.trim()) ??
        formHeaders.value.filter((header) => header.key.trim()),
      remotes: formRemotes.value
        .filter((remote) => remote.url.trim())
        .map((remote) => ({
          name: remote.name.trim(),
          url: remote.url.trim(),
          headers: remote.headers.filter((header) => header.key.trim()),
        })),
      hook: formHook.value.trim() || null,
      keep_subscription_groups_and_rules:
        formKeepSubscriptionGroupsAndRules.value,
      update_interval_hours: updateIntervalHours.value
        ? Number(updateIntervalHours.value)
        : null,
      update_cron: updateCron.value.trim() || null,
    };
    if (editingId.value) {
      await updateProfile(editingId.value, payload);
    } else {
      await createProfile(payload);
    }
    ensureProfiles(await refreshProfiles());
    toast.info({
      title: i18n.global.t(
        editingId.value ? 'profiles.updated' : 'profiles.added',
      ),
    });
    resetForm();
  } catch (error) {
    toast.error({
      content:
        error instanceof Error
          ? error.message
          : i18n.global.t('profiles.saveFailed'),
      title: i18n.global.t('profiles.operationFailed'),
    });
  } finally {
    saving.value = false;
  }
}

async function handleDelete(item: ProfileSummary) {
  if (
    !window.confirm(
      i18n.global.t('profiles.confirmDelete', { name: item.name }),
    )
  ) {
    return;
  }

  try {
    await deleteProfile(item.id);
    ensureProfiles(await refreshProfiles());
    toast.info({
      title: i18n.global.t('profiles.deleted', { name: item.name }),
    });
    if (editingId.value === item.id) {
      resetForm();
    }
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

async function handleSetCurrent(item: ProfileSummary) {
  if (item.id === currentProfileId.value || switchingProfile) {
    return;
  }

  try {
    switchingProfile = true;
    await setCurrentProfile(item.id);
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

async function handleRefresh(item: ProfileSummary) {
  try {
    ensureProfiles(await refreshProfileById(item.id));
    await refreshProfiles();
    await runtimeStatus.refetch();
    toast.info({
      title: i18n.global.t('profiles.refreshed', { name: item.name }),
    });
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

export default __render<ProfilesPageProps>(() => {
  return (
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
              onClick={openCreateDialog}
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
              onEdit={startEdit}
              onDelete={handleDelete}
              onEditNodes={openNodeEditor}
              onRefresh={handleRefresh}
              onSetCurrent={handleSetCurrent}
            />
          </PageContent>
        ),
        overlay: () => (
          <>
            <ProfileNodeEditor
              content={nodeEditorContent.value}
              loading={profileContentQuery.isFetching.value}
              open={nodeEditorOpen.value}
              profile={nodeEditorProfile.value}
              saving={saveProfileContentQuery.isFetching.value}
              onClose={closeNodeEditor}
              onContentChange={(content) => {
                nodeEditorContent.value = content;
              }}
              onSave={saveNodeEditorContent}
            />
          </>
        ),
      }}
    </Page>
  );
});
