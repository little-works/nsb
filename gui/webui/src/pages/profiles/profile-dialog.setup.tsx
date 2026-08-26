import { __render } from '@/shared/helpter';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { Select } from '@/components/select';
import { Icon } from '@/components/icon';
import { Page, PageContent } from '@/components/page-content';
import { CodeEditor } from '@/components/code-editor';
import { Dialog } from '@/components/dialog';
import { Popover } from '@/components/popover';
import { PROFILE_EDIT_SECTION_IDS } from '@/pages/profiles/profile-edit-sections';
import TemplateJsonEditor from '@/pages/profiles/profile-node-editor.setup';
import type { ProfileRemote, ProfileTemplate } from '@/types';
import { useMounted } from '@vueuse/core';
import {
  AddOutlined,
  ArrowDownwardOutlined,
  ArrowUpwardOutlined,
  CodeOutlined,
  DeleteOutlineOutlined,
  EditOutlined,
  ExpandLessOutlined,
  ExpandMoreOutlined,
  HelpOutlineOutlined,
  LinkOutlined,
  ScheduleOutlined,
} from '@vicons/material';
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';

const HOOK_EXAMPLE_CODE =
  "input.singbox.log ??= {};\ninput.singbox.log.level = 'debug';";

type JsonObject = Record<string, unknown>;

function serializeInlineTemplate(template: JsonObject | null) {
  return template ? JSON.stringify(template, null, 2) : '';
}

function parseInlineTemplate(content: string): JsonObject | null {
  const value: unknown = JSON.parse(content);
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as JsonObject)
    : null;
}

export interface ProfileDialogProps {
  open: boolean;
  editing: boolean;
  formName: string;
  remotes: ProfileRemote[];
  templateId: string;
  inlineTemplate: JsonObject | null;
  templates: ProfileTemplate[];
  hook: string;
  updateIntervalHours: string;
  updateCron: string;
  submitLabel: string;
  saving: boolean;
  onClose: () => void;
  onNameInput: (value: string) => void;
  onRemotesChange: (value: ProfileRemote[]) => void;
  onTemplateIdChange: (value: string) => void;
  onInlineTemplateChange: (value: JsonObject | null) => void;
  onHookInput: (value: string) => void;
  onUpdateCronInput: (value: string) => void;
  onUpdateIntervalInput: (value: string) => void;
  onSubmit: () => void | Promise<void>;
}

const props = defineProps<ProfileDialogProps>();

defineOptions({ name: 'ProfileDialog' });

const expandedHeaderIndexes = ref(new Set<number>());
const templateEditorOpen = ref(false);
const inlineTemplateDraft = ref<JsonObject | null>(null);
const templateViewerOpen = ref(false);
const keepFieldsRemoteIndex = ref<number | null>(null);
const mounted = useMounted();

function renderHookHelpButton(ariaLabel: string) {
  return (
    <Button
      aria-label={ariaLabel}
      iconOnly
      shape="square"
      size="xs"
      variant="ghost"
    >
      <Icon class="text-base">
        <HelpOutlineOutlined />
      </Icon>
    </Button>
  );
}

function updateRemote(index: number, next: Partial<ProfileRemote>) {
  props.onRemotesChange(
    props.remotes.map((remote, remoteIndex) =>
      remoteIndex === index ? { ...remote, ...next } : remote,
    ),
  );
}

function removeRemote(index: number) {
  props.onRemotesChange(
    props.remotes.filter((_, remoteIndex) => remoteIndex !== index),
  );
  expandedHeaderIndexes.value = new Set();
}

function moveRemote(index: number, direction: -1 | 1) {
  const targetIndex = index + direction;
  if (targetIndex < 0 || targetIndex >= props.remotes.length) return;
  const next = [...props.remotes];
  [next[index], next[targetIndex]] = [next[targetIndex], next[index]];
  props.onRemotesChange(next);
}

function addRemote() {
  props.onRemotesChange([
    ...props.remotes,
    {
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
    },
  ]);
}

function addHeader(remoteIndex: number) {
  updateRemote(remoteIndex, {
    headers: [...props.remotes[remoteIndex].headers, { key: '', value: '' }],
  });
  expandedHeaderIndexes.value = new Set([
    ...expandedHeaderIndexes.value,
    remoteIndex,
  ]);
}

function updateHeader(
  remoteIndex: number,
  headerIndex: number,
  next: Partial<ProfileRemote['headers'][number]>,
) {
  updateRemote(remoteIndex, {
    headers: props.remotes[remoteIndex].headers.map((header, index) =>
      index === headerIndex ? { ...header, ...next } : header,
    ),
  });
}

function removeHeader(remoteIndex: number, headerIndex: number) {
  updateRemote(remoteIndex, {
    headers: props.remotes[remoteIndex].headers.filter(
      (_, index) => index !== headerIndex,
    ),
  });
}

function toggleHeaders(index: number) {
  const next = new Set(expandedHeaderIndexes.value);
  if (next.has(index)) {
    next.delete(index);
  } else {
    next.add(index);
  }
  expandedHeaderIndexes.value = next;
}

function openInlineTemplateEditor() {
  inlineTemplateDraft.value = props.inlineTemplate;
  templateEditorOpen.value = true;
}

function saveInlineTemplate() {
  props.onInlineTemplateChange(inlineTemplateDraft.value);
  templateEditorOpen.value = false;
}

const { t } = useI18n();

export default __render<ProfileDialogProps>(() => {
  const keepRemote =
    keepFieldsRemoteIndex.value == null
      ? null
      : props.remotes[keepFieldsRemoteIndex.value];
  const remoteLabel = (remote: ProfileRemote, index: number) =>
    remote.name.trim() ||
    t('profiles.dialog.remotePlaceholder', { index: index + 1 });
  const remoteSummary = (remote: ProfileRemote) => {
    if (!remote.url.trim()) return t('profiles.dialog.remoteUnconfigured');
    try {
      return new URL(remote.url).host || remote.url;
    } catch {
      return remote.url;
    }
  };

  return (
    <>
      <Page
        title={t(
          props.editing
            ? 'profiles.dialog.editTitle'
            : 'profiles.dialog.createTitle',
        )}
        subtitle={t('profiles.dialog.subtitle')}
      >
        <PageContent class="max-w-4xl pb-24 pt-4">
          <section
            id={PROFILE_EDIT_SECTION_IDS.basicConfiguration}
            class="mb-10 scroll-mt-4"
          >
            <div class="mb-3 flex items-center gap-2">
              <Icon class="text-xl text-primary">
                <EditOutlined />
              </Icon>
              <h3 class="text-lg font-bold leading-6 text-on-surface">
                {t('profiles.dialog.basicConfiguration')}
              </h3>
            </div>
            <div
              class={[
                'overflow-hidden rounded border border-outline-variant',
                'bg-surface-container-lowest',
              ]}
            >
              <label class="block p-4">
                <span class="mb-1 block text-sm font-medium leading-5 text-on-surface">
                  {t('profiles.dialog.name')}
                </span>
                <span class="mb-3 block text-sm leading-5 text-on-surface-variant">
                  {t('profiles.dialog.nameDesc')}
                </span>
                <Input
                  size="sm"
                  value={props.formName}
                  onInput={(event) => {
                    props.onNameInput((event.target as HTMLInputElement).value);
                  }}
                  placeholder={t('profiles.dialog.namePlaceholder')}
                />
              </label>
              <div class="border-t border-outline-variant/50 p-4">
                <div class="mb-3">
                  <div>
                    <p class="text-sm font-medium text-on-surface">
                      {t('profiles.dialog.template')}
                    </p>
                    <p class="text-sm text-on-surface-variant">
                      {t('profiles.dialog.templateDesc')}
                    </p>
                  </div>
                </div>
                <div class="flex flex-wrap items-center gap-2">
                  <div class="min-w-48 flex-1">
                    <Select
                      ariaLabel={t('profiles.dialog.template')}
                      block
                      size="sm"
                      modelValue={props.templateId}
                      options={[
                        {
                          value: '',
                          label: t('profiles.dialog.inlineTemplate'),
                        },
                        ...props.templates.map((template) => ({
                          value: template.id,
                          label: template.name,
                        })),
                      ]}
                      onUpdateModelValue={props.onTemplateIdChange}
                    />
                  </div>
                  {props.templateId ? (
                    <Button
                      shape="rect"
                      size="sm"
                      variant="outline"
                      onClick={() => {
                        templateViewerOpen.value = true;
                      }}
                    >
                      {t('common.view')}
                    </Button>
                  ) : (
                    <Button
                      shape="rect"
                      size="sm"
                      variant="outline"
                      onClick={openInlineTemplateEditor}
                    >
                      {t('common.edit')}
                    </Button>
                  )}
                </div>
              </div>
            </div>
          </section>

          <section
            id={PROFILE_EDIT_SECTION_IDS.remoteSources}
            class="mb-10 scroll-mt-4"
          >
            <div class="mb-3 flex h-10 items-center justify-between gap-3">
              <div class="flex min-w-0 items-center gap-2">
                <Icon class="text-xl text-primary">
                  <LinkOutlined />
                </Icon>
                <h3 class="text-lg font-bold leading-6 text-on-surface">
                  {t('profiles.dialog.remoteSources')}
                </h3>
              </div>
              <Button
                shape="rect"
                size="xs"
                variant="ghost"
                onClick={addRemote}
              >
                <Icon class="text-base">
                  <AddOutlined />
                </Icon>
                {t('profiles.dialog.addRemote')}
              </Button>
            </div>
            <p class="mb-3 text-sm leading-5 text-on-surface-variant">
              {t('profiles.dialog.remoteSourcesDesc')}
            </p>
            <div class="space-y-3">
              {props.remotes.map((remote, index) => {
                const headersExpanded = expandedHeaderIndexes.value.has(index);

                return (
                  <section
                    key={index}
                    class={[
                      'overflow-hidden rounded border border-outline-variant',
                      'bg-surface-container-lowest',
                    ]}
                  >
                    <div
                      class={[
                        'flex h-12 items-center gap-3 px-3',
                        'border-b border-outline-variant/50',
                      ]}
                    >
                      <div class="min-w-0 flex-1">
                        <p class="truncate text-sm font-medium text-on-surface">
                          {remoteLabel(remote, index)}
                        </p>
                        <p class="truncate text-xs text-on-surface-variant">
                          {remoteSummary(remote)}
                        </p>
                      </div>
                      <Button
                        aria-label={t('profiles.dialog.moveRemoteUp', {
                          name: remoteLabel(remote, index),
                        })}
                        iconOnly
                        shape="square"
                        size="xs"
                        tooltip={t('profiles.dialog.moveRemoteUp', {
                          name: remoteLabel(remote, index),
                        })}
                        variant="ghost"
                        disabled={index === 0}
                        onClick={() => moveRemote(index, -1)}
                      >
                        <Icon class="text-base">
                          <ArrowUpwardOutlined />
                        </Icon>
                      </Button>
                      <Button
                        aria-label={t('profiles.dialog.moveRemoteDown', {
                          name: remoteLabel(remote, index),
                        })}
                        iconOnly
                        shape="square"
                        size="xs"
                        tooltip={t('profiles.dialog.moveRemoteDown', {
                          name: remoteLabel(remote, index),
                        })}
                        variant="ghost"
                        disabled={index === props.remotes.length - 1}
                        onClick={() => moveRemote(index, 1)}
                      >
                        <Icon class="text-base">
                          <ArrowDownwardOutlined />
                        </Icon>
                      </Button>
                      <Button
                        aria-label={t('profiles.dialog.removeRemote', {
                          name: remoteLabel(remote, index),
                        })}
                        iconOnly
                        shape="square"
                        size="xs"
                        tooltip={t('profiles.dialog.removeRemote', {
                          name: remoteLabel(remote, index),
                        })}
                        variant="danger-ghost"
                        disabled={props.remotes.length === 1}
                        onClick={() => removeRemote(index)}
                      >
                        <Icon class="text-base">
                          <DeleteOutlineOutlined />
                        </Icon>
                      </Button>
                    </div>

                    <div class="space-y-3 p-3">
                      <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]">
                        <label class="block min-w-0">
                          <span class="mb-1 block text-xs font-medium text-on-surface-variant">
                            {t('profiles.dialog.remoteName')}
                          </span>
                          <Input
                            size="sm"
                            placeholder={t(
                              'profiles.dialog.remotePlaceholder',
                              {
                                index: index + 1,
                              },
                            )}
                            value={remote.name}
                            onInput={(event) =>
                              updateRemote(index, {
                                name: (event.target as HTMLInputElement).value,
                              })
                            }
                          />
                        </label>
                        <label class="block min-w-0">
                          <span class="mb-1 block text-xs font-medium text-on-surface-variant">
                            {t('profiles.dialog.remoteFormat')}
                          </span>
                          <Select
                            ariaLabel={t('profiles.dialog.remoteFormat')}
                            block
                            size="sm"
                            modelValue={remote.format}
                            options={[
                              {
                                value: 'clash',
                                label: t('profiles.dialog.remoteFormatClash'),
                              },
                              {
                                value: 'singbox',
                                label: t('profiles.dialog.remoteFormatSingbox'),
                              },
                            ]}
                            onUpdateModelValue={(value) =>
                              updateRemote(index, {
                                format: value as ProfileRemote['format'],
                              })
                            }
                          />
                        </label>
                        <div>
                          <span class="mb-1 block text-xs font-medium text-on-surface-variant">
                            {t('profiles.dialog.keepFields')}
                          </span>
                          <Button
                            shape="rect"
                            size="sm"
                            variant="outline"
                            onClick={() => {
                              keepFieldsRemoteIndex.value = index;
                            }}
                          >
                            {t('profiles.dialog.keepFields')}
                          </Button>
                        </div>
                      </div>
                      <label class="block">
                        <span class="mb-1 block text-xs font-medium text-on-surface-variant">
                          {t('profiles.dialog.remoteUrl')}
                        </span>
                        <Input
                          size="sm"
                          placeholder={t(
                            'profiles.dialog.remoteUrlPlaceholder',
                          )}
                          value={remote.url}
                          onInput={(event) =>
                            updateRemote(index, {
                              url: (event.target as HTMLInputElement).value,
                            })
                          }
                        />
                      </label>
                      <div class="hidden">
                        <p class="mb-2 text-xs font-medium text-on-surface-variant">
                          {t('profiles.dialog.keepFields')}
                        </p>
                        <div class="space-y-2">
                          <label class="flex items-center gap-2 text-sm text-on-surface">
                            <input
                              class="h-3.5 w-3.5 accent-primary"
                              type="checkbox"
                              checked={remote.keep.nodes}
                              onChange={(event) =>
                                updateRemote(index, {
                                  keep: {
                                    ...remote.keep,
                                    nodes: (event.target as HTMLInputElement)
                                      .checked,
                                  },
                                })
                              }
                            />
                            {t(
                              remote.format === 'clash'
                                ? 'profiles.dialog.keepClashProxies'
                                : 'profiles.dialog.keepSingboxOutbounds',
                            )}
                          </label>
                          {remote.format === 'clash' ? (
                            <>
                              <label class="flex items-center gap-2 text-sm text-on-surface">
                                <input
                                  class="h-3.5 w-3.5 accent-primary"
                                  type="checkbox"
                                  checked={remote.keep.groups}
                                  onChange={(event) =>
                                    updateRemote(index, {
                                      keep: {
                                        ...remote.keep,
                                        groups: (
                                          event.target as HTMLInputElement
                                        ).checked,
                                      },
                                    })
                                  }
                                />
                                {t('profiles.dialog.keepClashProxyGroups')}
                              </label>
                              <label class="flex items-center gap-2 text-sm text-on-surface">
                                <input
                                  class="h-3.5 w-3.5 accent-primary"
                                  type="checkbox"
                                  checked={remote.keep.route_rules}
                                  onChange={(event) =>
                                    updateRemote(index, {
                                      keep: {
                                        ...remote.keep,
                                        route_rules: (
                                          event.target as HTMLInputElement
                                        ).checked,
                                      },
                                    })
                                  }
                                />
                                {t('profiles.dialog.keepClashRules')}
                              </label>
                            </>
                          ) : (
                            <div class="ml-3 space-y-2 border-l border-outline-variant pl-3">
                              <p class="text-xs font-medium text-on-surface-variant">
                                {t('profiles.dialog.keepSingboxRoute')}
                              </p>
                              <label class="flex items-center gap-2 text-sm text-on-surface">
                                <input
                                  class="h-3.5 w-3.5 accent-primary"
                                  type="checkbox"
                                  checked={remote.keep.route_final}
                                  onChange={(event) =>
                                    updateRemote(index, {
                                      keep: {
                                        ...remote.keep,
                                        route_final: (
                                          event.target as HTMLInputElement
                                        ).checked,
                                      },
                                    })
                                  }
                                />
                                {t('profiles.dialog.keepSingboxFinal')}
                              </label>
                              <label class="flex items-center gap-2 text-sm text-on-surface">
                                <input
                                  class="h-3.5 w-3.5 accent-primary"
                                  type="checkbox"
                                  checked={remote.keep.route_rules}
                                  onChange={(event) =>
                                    updateRemote(index, {
                                      keep: {
                                        ...remote.keep,
                                        route_rules: (
                                          event.target as HTMLInputElement
                                        ).checked,
                                      },
                                    })
                                  }
                                />
                                {t('profiles.dialog.keepSingboxRules')}
                              </label>
                            </div>
                          )}
                        </div>
                      </div>
                    </div>

                    <div class="border-t border-outline-variant/50">
                      <div class="flex h-10 items-center gap-2 px-3">
                        <button
                          class={[
                            'flex min-w-0 flex-1 items-center gap-1.5',
                            'text-left text-xs font-medium text-on-surface-variant',
                            'hover:text-primary',
                          ]}
                          type="button"
                          onClick={() => toggleHeaders(index)}
                        >
                          <Icon class="text-base">
                            {headersExpanded ? (
                              <ExpandLessOutlined />
                            ) : (
                              <ExpandMoreOutlined />
                            )}
                          </Icon>
                          {t('profiles.dialog.headers', {
                            count: remote.headers.length,
                          })}
                        </button>
                        <Button
                          shape="rect"
                          size="xs"
                          variant="ghost"
                          onClick={() => addHeader(index)}
                        >
                          <Icon class="text-base">
                            <AddOutlined />
                          </Icon>
                          {t('profiles.dialog.addHeader')}
                        </Button>
                      </div>

                      {headersExpanded ? (
                        <div class="border-t border-outline-variant/50 p-3">
                          {remote.headers.length ? (
                            <div class="space-y-2">
                              <div
                                class={[
                                  'grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_1.75rem] gap-2 px-1',
                                  'text-xs font-medium text-on-surface-variant',
                                ]}
                              >
                                <span>{t('profiles.dialog.headerKey')}</span>
                                <span>{t('profiles.dialog.headerValue')}</span>
                              </div>
                              {remote.headers.map((header, headerIndex) => (
                                <div
                                  key={headerIndex}
                                  class="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_1.75rem] items-center gap-2"
                                >
                                  <Input
                                    size="sm"
                                    class="min-w-0 px-2 text-xs"
                                    placeholder={t(
                                      'profiles.dialog.headerKeyPlaceholder',
                                    )}
                                    value={header.key}
                                    onInput={(event) =>
                                      updateHeader(index, headerIndex, {
                                        key: (event.target as HTMLInputElement)
                                          .value,
                                      })
                                    }
                                  />
                                  <Input
                                    size="sm"
                                    class="min-w-0 px-2 text-xs"
                                    placeholder={t(
                                      'profiles.dialog.headerValue',
                                    )}
                                    value={header.value}
                                    onInput={(event) =>
                                      updateHeader(index, headerIndex, {
                                        value: (
                                          event.target as HTMLInputElement
                                        ).value,
                                      })
                                    }
                                  />
                                  <Button
                                    aria-label={t(
                                      'profiles.dialog.removeHeader',
                                    )}
                                    iconOnly
                                    shape="square"
                                    size="sm"
                                    tooltip={t('profiles.dialog.removeHeader')}
                                    variant="danger-ghost"
                                    onClick={() =>
                                      removeHeader(index, headerIndex)
                                    }
                                  >
                                    <Icon class="text-base">
                                      <DeleteOutlineOutlined />
                                    </Icon>
                                  </Button>
                                </div>
                              ))}
                            </div>
                          ) : (
                            <p class="py-1 text-xs text-on-surface-variant">
                              {t('profiles.dialog.noHeaders')}
                            </p>
                          )}
                        </div>
                      ) : null}
                    </div>
                  </section>
                );
              })}
            </div>
          </section>

          <section
            id={PROFILE_EDIT_SECTION_IDS.updateSchedule}
            class="mb-10 scroll-mt-4"
          >
            <div class="mb-3 flex items-center gap-2">
              <Icon class="text-xl text-primary">
                <ScheduleOutlined />
              </Icon>
              <h3 class="text-lg font-bold leading-6 text-on-surface">
                {t('profiles.dialog.updateSchedule')}
              </h3>
            </div>
            <div
              class={[
                'grid overflow-hidden rounded border border-outline-variant',
                'bg-surface-container-lowest sm:grid-cols-2',
              ]}
            >
              <label class="border-b border-outline-variant/50 p-4 sm:border-b-0 sm:border-r">
                <span class="block text-sm font-medium leading-5 text-on-surface">
                  {t('profiles.dialog.updateInterval')}
                </span>
                <span class="mb-3 block text-sm leading-5 text-on-surface-variant">
                  {t('profiles.dialog.updateIntervalDesc')}
                </span>
                <Input
                  size="sm"
                  min="1"
                  type="number"
                  value={props.updateIntervalHours}
                  onInput={(event) =>
                    props.onUpdateIntervalInput(
                      (event.target as HTMLInputElement).value,
                    )
                  }
                />
              </label>
              <label class="block p-4">
                <span class="block text-sm font-medium leading-5 text-on-surface">
                  {t('profiles.dialog.updateCron')}
                </span>
                <span class="mb-3 block text-sm leading-5 text-on-surface-variant">
                  {t('profiles.dialog.updateCronDesc')}
                </span>
                <Input
                  size="sm"
                  class="font-mono"
                  value={props.updateCron}
                  placeholder={t('profiles.dialog.updateCronPlaceholder')}
                  onInput={(event) =>
                    props.onUpdateCronInput(
                      (event.target as HTMLInputElement).value,
                    )
                  }
                />
              </label>
            </div>
          </section>

          <section
            id={PROFILE_EDIT_SECTION_IDS.customHook}
            class="mb-10 scroll-mt-4"
          >
            <div class="mb-3 flex items-center gap-2">
              <Icon class="text-xl text-primary">
                <CodeOutlined />
              </Icon>
              <h3 class="text-lg font-bold leading-6 text-on-surface">
                {t('profiles.dialog.customHook')}
              </h3>
              {mounted.value && (
                <Popover
                  trigger="click"
                  placement="right-start"
                  offset={8}
                  contentClass={[
                    'z-dropdown w-80 rounded border border-outline-variant p-4',
                    'bg-surface-container-lowest text-on-surface shadow-xl',
                  ]}
                  v-slots={{
                    default: () =>
                      renderHookHelpButton(
                        t('profiles.dialog.hookHelpAriaLabel'),
                      ),
                    overlay: () => (
                      <div class="space-y-3">
                        <div>
                          <p class="text-sm font-semibold text-on-surface">
                            {t('profiles.dialog.hookHelpTitle')}
                          </p>
                          <p class="mt-1 text-xs leading-5 text-on-surface-variant">
                            {t('profiles.dialog.hookGenerateIntro')}
                            <code class="rounded bg-surface-container px-1 font-mono text-on-surface">
                              {t('profiles.dialog.hookGenerateFunction')}
                            </code>
                            {t('profiles.dialog.hookGenerateReturns')}
                          </p>
                          <p class="mt-1 text-xs leading-5 text-on-surface-variant">
                            {t('profiles.dialog.hookFinalizeIntro')}
                            <code class="rounded bg-surface-container px-1 font-mono text-on-surface">
                              {t('profiles.dialog.hookFinalizeFunction')}
                            </code>
                            {t('profiles.dialog.hookFinalizeReturns')}
                          </p>
                        </div>
                        <div class="text-xs leading-5 text-on-surface-variant">
                          <p>
                            <code class="font-mono text-on-surface">
                              {t('profiles.dialog.hookSingbox')}
                            </code>
                            {t('profiles.dialog.hookFinalizeSingbox')}
                          </p>
                          <p>
                            {t('profiles.dialog.hookHelpSingle')}
                            <code class="font-mono text-on-surface">
                              {' '}
                              {t('profiles.dialog.hookRemote')}{' '}
                            </code>
                            {t('profiles.dialog.hookHelpMultiple')}
                            <code class="font-mono text-on-surface">
                              {' '}
                              {t('profiles.dialog.hookRemotes')}{' '}
                            </code>
                            {t('profiles.dialog.hookHelpRemoteData')}
                          </p>
                        </div>
                        <div>
                          <p class="mb-1 text-xs font-medium text-on-surface-variant">
                            {t('profiles.dialog.hookExample')}
                          </p>
                          <pre class="overflow-x-auto rounded bg-surface-container p-2 font-mono text-xs leading-5 text-on-surface">
                            {HOOK_EXAMPLE_CODE}
                          </pre>
                        </div>
                      </div>
                    ),
                  }}
                />
              )}
            </div>
            <div
              class={[
                'overflow-hidden rounded border border-outline-variant',
                'bg-surface-container-lowest p-4',
              ]}
            >
              <p class="mb-3 text-sm leading-5 text-on-surface-variant">
                {t('profiles.dialog.customHookDesc')}
              </p>
              <div
                class="
                h-48 min-h-32 max-h-[70vh]
                resize-y overflow-hidden
                rounded border border-outline-variant
              "
              >
                <CodeEditor
                  value={props.hook}
                  language="javascript"
                  onChange={props.onHookInput}
                />
              </div>
            </div>
          </section>

          <div class="flex h-12 justify-end gap-2">
            <Button
              shape="rect"
              size="field"
              variant="outline"
              disabled={props.saving}
              onClick={props.onClose}
            >
              {t('common.cancel')}
            </Button>
            <Button
              shape="rect"
              size="field"
              variant="solid"
              disabled={props.saving}
              onClick={props.onSubmit}
            >
              {props.submitLabel}
            </Button>
          </div>
        </PageContent>
      </Page>
      <TemplateJsonEditor
        content={serializeInlineTemplate(inlineTemplateDraft.value)}
        loading={false}
        open={templateEditorOpen.value}
        saveLabel={t('common.save')}
        saving={false}
        title={t('profiles.dialog.editInlineTemplate')}
        onClose={() => {
          templateEditorOpen.value = false;
        }}
        onContentChange={(value) => {
          inlineTemplateDraft.value = parseInlineTemplate(value);
        }}
        onSave={saveInlineTemplate}
      />
      <Dialog
        open={templateViewerOpen.value}
        title={t('profiles.dialog.viewTemplate')}
        contentClass="max-w-4xl"
        onClose={() => {
          templateViewerOpen.value = false;
        }}
      >
        <pre class="max-h-[70vh] overflow-auto rounded bg-surface-container-low p-3 font-mono text-xs text-on-surface">
          {props.templates.find((template) => template.id === props.templateId)
            ?.content ?? ''}
        </pre>
      </Dialog>
      <Dialog
        open={keepRemote != null}
        title={t('profiles.dialog.keepFields')}
        contentClass="max-w-md"
        onClose={() => {
          keepFieldsRemoteIndex.value = null;
        }}
      >
        {keepRemote != null && keepFieldsRemoteIndex.value != null ? (
          <div class="space-y-3">
            <label class="flex items-center gap-2 text-sm text-on-surface">
              <input
                class="h-3.5 w-3.5 accent-primary"
                type="checkbox"
                checked={keepRemote.keep.nodes}
                onChange={(event) =>
                  updateRemote(keepFieldsRemoteIndex.value!, {
                    keep: {
                      ...keepRemote.keep,
                      nodes: (event.target as HTMLInputElement).checked,
                    },
                  })
                }
              />
              {t(
                keepRemote.format === 'clash'
                  ? 'profiles.dialog.keepClashProxies'
                  : 'profiles.dialog.keepSingboxOutbounds',
              )}
            </label>
            {keepRemote.format === 'clash' ? (
              <>
                <label class="flex items-center gap-2 text-sm text-on-surface">
                  <input
                    class="h-3.5 w-3.5 accent-primary"
                    type="checkbox"
                    checked={keepRemote.keep.groups}
                    onChange={(event) =>
                      updateRemote(keepFieldsRemoteIndex.value!, {
                        keep: {
                          ...keepRemote.keep,
                          groups: (event.target as HTMLInputElement).checked,
                        },
                      })
                    }
                  />
                  {t('profiles.dialog.keepClashProxyGroups')}
                </label>
                <label class="flex items-center gap-2 text-sm text-on-surface">
                  <input
                    class="h-3.5 w-3.5 accent-primary"
                    type="checkbox"
                    checked={keepRemote.keep.route_rules}
                    onChange={(event) =>
                      updateRemote(keepFieldsRemoteIndex.value!, {
                        keep: {
                          ...keepRemote.keep,
                          route_rules: (event.target as HTMLInputElement)
                            .checked,
                        },
                      })
                    }
                  />
                  {t('profiles.dialog.keepClashRules')}
                </label>
              </>
            ) : (
              <>
                <label class="flex items-center gap-2 text-sm text-on-surface">
                  <input
                    class="h-3.5 w-3.5 accent-primary"
                    type="checkbox"
                    checked={keepRemote.keep.route_final}
                    onChange={(event) =>
                      updateRemote(keepFieldsRemoteIndex.value!, {
                        keep: {
                          ...keepRemote.keep,
                          route_final: (event.target as HTMLInputElement)
                            .checked,
                        },
                      })
                    }
                  />
                  {t('profiles.dialog.keepSingboxFinal')}
                </label>
                <label class="flex items-center gap-2 text-sm text-on-surface">
                  <input
                    class="h-3.5 w-3.5 accent-primary"
                    type="checkbox"
                    checked={keepRemote.keep.route_rules}
                    onChange={(event) =>
                      updateRemote(keepFieldsRemoteIndex.value!, {
                        keep: {
                          ...keepRemote.keep,
                          route_rules: (event.target as HTMLInputElement)
                            .checked,
                        },
                      })
                    }
                  />
                  {t('profiles.dialog.keepSingboxRules')}
                </label>
              </>
            )}
          </div>
        ) : null}
      </Dialog>
    </>
  );
});
