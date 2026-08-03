import { __render } from '@/shared/helpter';
import { createProfile, updateProfile } from '@/api/client';
import { toast } from '@/components/toast';
import ProfileDialog from '@/pages/profiles/profile-dialog.setup';
import { useAppSnapshot } from '@/store/app';
import type { ProfileHeader, ProfileRemote } from '@/types';
import { computed, ref, watchEffect } from 'vue';
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

const appSnapshot = useAppSnapshot();
const pageContext = usePageContext();
const profileId = new URL(
  pageContext.urlOriginal,
  'http://localhost',
).searchParams.get('id');
const initialized = ref(false);
const saving = ref(false);
const name = ref('');
const remotes = ref<ProfileRemote[]>([{ name: '', url: '', headers: [] }]);
const hook = ref(EMPTY_PROFILE_HOOK_TEMPLATE);
const interval = ref('');
const cron = ref('');

const profile = computed(() =>
  (appSnapshot.data.value?.state.gui_config.profiles ?? []).find(
    (item) => item.id === profileId,
  ),
);

watchEffect(() => {
  if (!profile.value || initialized.value) return;
  name.value = profile.value.name;
  remotes.value = (
    profile.value.remotes.length
      ? profile.value.remotes
      : [{ name: '', url: profile.value.url, headers: profile.value.headers }]
  ).map((remote) => ({ ...remote, headers: [...remote.headers] }));
  hook.value = profile.value.hook || EMPTY_PROFILE_HOOK_TEMPLATE;
  interval.value = profile.value.update_interval_hours?.toString() ?? '';
  cron.value = profile.value.update_cron ?? '';
  initialized.value = true;
});

function close() {
  void navigate('/webui/profiles');
}

async function submit() {
  saving.value = true;
  try {
    const payload = {
      name: name.value,
      source: remotes.value[0]?.url ?? '',
      content: null,
      headers: remotes.value[0]?.headers ?? [],
      remotes: remotes.value.filter((remote) => remote.url.trim()),
      hook: hook.value.trim() || null,
      update_interval_hours: interval.value ? Number(interval.value) : null,
      update_cron: cron.value.trim() || null,
    };
    if (profileId) await updateProfile(profileId, payload);
    else await createProfile(payload);
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
    editing={Boolean(profileId)}
    formName={name.value}
    formSource=""
    headers={[]}
    remotes={remotes.value}
    hook={hook.value}
    updateIntervalHours={interval.value}
    updateCron={cron.value}
    submitLabel={profileId ? 'Save Changes' : 'Add Profile'}
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
