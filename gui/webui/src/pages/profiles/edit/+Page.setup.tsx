import { __render } from '@/shared/helpter';
import { createProfile, getProfile, updateProfile } from '@/api/client';
import { Button } from '@/components/button';
import { Icon } from '@/components/icon';
import { toast } from '@/components/toast';
import ProfileDialog from '@/pages/profiles/profile-dialog.setup';
import { PROFILE_EDIT_SECTION_IDS } from '@/pages/profiles/profile-edit-sections';
import { useProfiles, useRuntimeStatus } from '@/store/app';
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

const profilesQuery = useProfiles();
const runtimeStatus = useRuntimeStatus();
const floatingDockStore = useFloatingDockStore();
const pageContext = usePageContext();
const profileId = computed(() =>
  new URL(pageContext.urlOriginal, 'http://localhost').searchParams.get('id'),
);
const initialized = ref(false);
const saving = ref(false);
const name = ref('');
const remotes = ref<ProfileRemote[]>([{ name: '', url: '', headers: [] }]);
const hook = ref(EMPTY_PROFILE_HOOK_TEMPLATE);
const keepSubscriptionGroupsAndRules = ref(false);
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
  remotes.value = [{ name: '', url: '', headers: [] }];
  hook.value = EMPTY_PROFILE_HOOK_TEMPLATE;
  keepSubscriptionGroupsAndRules.value = false;
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
  remotes.value = (
    value.remotes.length
      ? value.remotes
      : [{ name: '', url: value.url, headers: value.headers }]
  ).map((remote) => ({ ...remote, headers: [...remote.headers] }));
  hook.value = value.hook || EMPTY_PROFILE_HOOK_TEMPLATE;
  keepSubscriptionGroupsAndRules.value =
    value.keep_subscription_groups_and_rules;
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
  const creatingFirstProfile =
    !profileId.value && !profilesQuery.data.value?.current_profile_id;
  saving.value = true;
  try {
    const payload = {
      name: name.value,
      source: remotes.value[0]?.url ?? '',
      content: null,
      headers: remotes.value[0]?.headers ?? [],
      remotes: remotes.value.filter((remote) => remote.url.trim()),
      hook: hook.value.trim() || null,
      keep_subscription_groups_and_rules: keepSubscriptionGroupsAndRules.value,
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

export default __render(() => (
  <ProfileDialog
    open
    editing={Boolean(profileId.value)}
    formName={name.value}
    formSource=""
    headers={[]}
    remotes={remotes.value}
    hook={hook.value}
    keepSubscriptionGroupsAndRules={keepSubscriptionGroupsAndRules.value}
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
    onSourceInput={() => {}}
    onRemotesChange={(value) => {
      remotes.value = value;
    }}
    onHookInput={(value) => {
      hook.value = value;
    }}
    onKeepSubscriptionGroupsAndRulesChange={(value) => {
      keepSubscriptionGroupsAndRules.value = value;
    }}
    onAddHeader={() => {}}
    onHeaderChange={() => {}}
    onRemoveHeader={() => {}}
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
