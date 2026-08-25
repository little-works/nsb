import { __render } from '@/shared/helpter';
import {
  createProfile,
  createTemplate,
  getDefaultTemplate,
  getProfile,
  updateProfile,
} from '@/api/client';
import { Button } from '@/components/button';
import { Icon } from '@/components/icon';
import { toast } from '@/components/toast';
import ProfileDialog from '@/pages/profiles/profile-dialog.setup';
import { PROFILE_EDIT_SECTION_IDS } from '@/pages/profiles/profile-edit-sections';
import { useProfiles, useRuntimeStatus, useTemplates } from '@/store/app';
import { useClientQuery } from '@/hooks/use-client-query';
import { useFloatingDockStore } from '@/store/floating-dock';
import type { ProfileRemote } from '@/types';
import {
  CodeOutlined,
  EditOutlined,
  LinkOutlined,
  ScheduleOutlined,
} from '@vicons/material';
import {
  computed,
  onActivated,
  onBeforeUnmount,
  onDeactivated,
  onMounted,
  ref,
  watch,
} from 'vue';
import { usePageContext } from 'vike-vue/usePageContext';
import { navigate } from 'vike/client/router';
import { i18n } from '@/i18n';

defineOptions({ name: 'ProfileEditPage' });

const EMPTY_PROFILE_HOOK_TEMPLATE = `export function onGenerate(input) {
  return input.singbox;
}

export function onFinalize(input) {
  return input.singbox;
}
`;

function createEmptyRemote(): ProfileRemote {
  return {
    name: '',
    url: '',
    headers: [],
    format: 'clash',
    keep: {
      nodes: true,
      groups: false,
      route_final: false,
      route_rules: false,
    },
  };
}

const profilesQuery = useProfiles();
const templatesQuery = useTemplates();
const runtimeStatus = useRuntimeStatus();
const floatingDockStore = useFloatingDockStore();
const pageContext = usePageContext();
const profileId = computed(() =>
  new URL(pageContext.urlOriginal, 'http://localhost').searchParams.get('id'),
);
const initialized = ref(false);
const saving = ref(false);
const name = ref('');
const templateId = ref('');
const remotes = ref<ProfileRemote[]>([createEmptyRemote()]);
const hook = ref(EMPTY_PROFILE_HOOK_TEMPLATE);
const interval = ref('');
const cron = ref('');
let unregisterDockContent: (() => void) | null = null;
let profileEditDockActive = false;

const profileQuery = useClientQuery(
  computed(() => ({
    queryKey: ['profile', profileId.value],
    enabled: Boolean(profileId.value),
    queryFn: async () => {
      const id = profileId.value;
      if (!id) throw new Error(i18n.global.t('profiles.noneSelected'));
      return getProfile(id);
    },
  })),
);
const profile = computed(() => {
  const value = profileQuery.data.value;
  return value?.id === profileId.value ? value : null;
});

function resetForm() {
  name.value = '';
  templateId.value = '';
  remotes.value = [createEmptyRemote()];
  hook.value = EMPTY_PROFILE_HOOK_TEMPLATE;
  interval.value = '';
  cron.value = '';
}

watch(
  profileId,
  () => {
    initialized.value = false;
    resetForm();
  },
  { immediate: true },
);

watch(profile, (value) => {
  if (!value || initialized.value) return;
  name.value = value.name;
  templateId.value = value.template_id;
  remotes.value = value.remotes.map((remote) => ({
    ...remote,
    headers: [...remote.headers],
    keep: { ...remote.keep },
  }));
  hook.value = value.hook || EMPTY_PROFILE_HOOK_TEMPLATE;
  interval.value = value.update_interval_hours?.toString() ?? '';
  cron.value = value.update_cron ?? '';
  initialized.value = true;
});

function close() {
  void navigate('/webui/profiles');
}

function scrollToSection(id: string) {
  document
    .getElementById(id)
    ?.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

function activateProfileEditDock() {
  if (profileEditDockActive) {
    return;
  }

  profileEditDockActive = true;
  unregisterDockContent = floatingDockStore.register(() => (
    <>
      <Button
        aria-label={i18n.global.t('profiles.dialog.basicConfiguration')}
        iconOnly
        shape="square"
        size="sm"
        tooltip={i18n.global.t('profiles.dialog.basicConfiguration')}
        variant="ghost"
        onClick={() =>
          scrollToSection(PROFILE_EDIT_SECTION_IDS.basicConfiguration)
        }
      >
        <Icon class="text-base">
          <EditOutlined />
        </Icon>
      </Button>
      <Button
        aria-label={i18n.global.t('profiles.dialog.remoteSources')}
        iconOnly
        shape="square"
        size="sm"
        tooltip={i18n.global.t('profiles.dialog.remoteSources')}
        variant="ghost"
        onClick={() => scrollToSection(PROFILE_EDIT_SECTION_IDS.remoteSources)}
      >
        <Icon class="text-base">
          <LinkOutlined />
        </Icon>
      </Button>
      <Button
        aria-label={i18n.global.t('profiles.dialog.updateSchedule')}
        iconOnly
        shape="square"
        size="sm"
        tooltip={i18n.global.t('profiles.dialog.updateSchedule')}
        variant="ghost"
        onClick={() => scrollToSection(PROFILE_EDIT_SECTION_IDS.updateSchedule)}
      >
        <Icon class="text-base">
          <ScheduleOutlined />
        </Icon>
      </Button>
      <Button
        aria-label={i18n.global.t('profiles.dialog.customHook')}
        iconOnly
        shape="square"
        size="sm"
        tooltip={i18n.global.t('profiles.dialog.customHook')}
        variant="ghost"
        onClick={() => scrollToSection(PROFILE_EDIT_SECTION_IDS.customHook)}
      >
        <Icon class="text-base">
          <CodeOutlined />
        </Icon>
      </Button>
    </>
  ));
}

function deactivateProfileEditDock() {
  profileEditDockActive = false;
  unregisterDockContent?.();
  unregisterDockContent = null;
}

onMounted(activateProfileEditDock);
onActivated(activateProfileEditDock);
onDeactivated(deactivateProfileEditDock);
onBeforeUnmount(deactivateProfileEditDock);

async function submit() {
  if (!templateId.value) {
    toast.error({ title: i18n.global.t('profiles.dialog.templateRequired') });
    return;
  }
  const creatingFirstProfile =
    !profileId.value && !profilesQuery.data.value?.current_profile_id;
  saving.value = true;
  try {
    const payload = {
      name: name.value,
      template_id: templateId.value,
      remotes: remotes.value
        .filter((remote) => remote.url.trim())
        .map((remote) => ({
          ...remote,
          name: remote.name.trim(),
          url: remote.url.trim(),
          headers: remote.headers.filter((header) => header.key.trim()),
        })),
      hook: hook.value.trim() || null,
      update_interval_hours: interval.value ? Number(interval.value) : null,
      update_cron: cron.value.trim() || null,
    };
    if (profileId.value) await updateProfile(profileId.value, payload);
    else await createProfile(payload);
    const result = await profilesQuery.refetch();
    if (!result.data) {
      throw new Error(i18n.global.t('profiles.loadFailed'));
    }
    if (creatingFirstProfile) {
      await runtimeStatus.refetch();
    }
    close();
  } catch (error) {
    toast.error({
      title:
        error instanceof Error
          ? error.message
          : i18n.global.t('profiles.saveFailed'),
    });
  } finally {
    saving.value = false;
  }
}

async function createPresetTemplate() {
  try {
    const content = await getDefaultTemplate();
    const template = await createTemplate({
      name: i18n.global.t('templates.defaultName'),
      content,
    });
    await templatesQuery.refetch();
    templateId.value = template.id;
  } catch (error) {
    toast.error({
      title:
        error instanceof Error
          ? error.message
          : i18n.global.t('templates.createFailed'),
    });
  }
}

export default __render(() => (
  <ProfileDialog
    open
    editing={Boolean(profileId.value)}
    formName={name.value}
    remotes={remotes.value}
    templateId={templateId.value}
    templates={templatesQuery.data.value ?? []}
    hook={hook.value}
    updateIntervalHours={interval.value}
    updateCron={cron.value}
    submitLabel={i18n.global.t(
      profileId.value ? 'profiles.saveChanges' : 'profiles.add',
    )}
    saving={saving.value}
    onClose={close}
    onNameInput={(value) => {
      name.value = value;
    }}
    onRemotesChange={(value) => {
      remotes.value = value;
    }}
    onTemplateIdChange={(value) => {
      templateId.value = value;
    }}
    onCreatePresetTemplate={createPresetTemplate}
    onHookInput={(value) => {
      hook.value = value;
    }}
    onUpdateCronInput={(value) => {
      cron.value = value;
      if (value.trim()) interval.value = '';
    }}
    onUpdateIntervalInput={(value) => {
      interval.value = value;
      if (value) cron.value = '';
    }}
    onSubmit={submit}
  />
));
