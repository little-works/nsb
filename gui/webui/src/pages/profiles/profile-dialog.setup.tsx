import { __render } from '@/shared/helpter';
import { Button } from '@/components/button';
import { Input } from '@/components/input';
import { Select } from '@/components/select';
import { Icon } from '@/components/icon';
import { Page, PageContent } from '@/components/page-content';
import { CodeEditor } from '@/components/code-editor';
import { Dialog } from '@/components/dialog';
import { Popover } from '@/components/popover';
import { Checkbox } from '@/components/checkbox';
import { Radio } from '@/components/radio';
import { PROFILE_EDIT_SECTION_IDS } from '@/pages/profiles/profile-edit-sections';
import { createDefaultProfileRemoteKeepFields } from '@/pages/profiles/profile-keep-fields';
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
import { ref, toRaw, type VNode } from 'vue';
import { useI18n } from 'vue-i18n';

const HOOK_EXAMPLE_CODE =
  "input.singbox.log ??= {};\ninput.singbox.log.level = 'debug';";

type JsonObject = Record<string, unknown>;

interface KeepFieldLeaf {
  id: string;
  label: string;
  checked: boolean;
  setChecked: (keep: ProfileRemote['keep'], checked: boolean) => void;
}

interface KeepFieldBranch {
  id: string;
  label: string;
  children: KeepFieldNode[];
}

type KeepFieldNode = KeepFieldLeaf | KeepFieldBranch;

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
const keepFieldsDraft = ref<ProfileRemote['keep'] | null>(null);
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

function updateKeepFields(leaves: KeepFieldLeaf[], checked: boolean) {
  const keep = keepFieldsDraft.value;
  if (!keep) return;
  for (const leaf of leaves) leaf.setChecked(keep, checked);
}

function cloneKeepFields(keep: ProfileRemote['keep']): ProfileRemote['keep'] {
  return JSON.parse(JSON.stringify(keep));
}

function openKeepFields(index: number) {
  const remote = props.remotes[index];
  if (!remote) return;
  keepFieldsDraft.value = cloneKeepFields(toRaw(remote.keep));
  keepFieldsRemoteIndex.value = index;
}

function closeKeepFields() {
  keepFieldsRemoteIndex.value = null;
  keepFieldsDraft.value = null;
}

function saveKeepFields() {
  const index = keepFieldsRemoteIndex.value;
  const keep = keepFieldsDraft.value;
  if (index == null || !keep) {
    closeKeepFields();
    return;
  }
  updateRemote(index, { keep: cloneKeepFields(toRaw(keep)) });
  closeKeepFields();
}

function descendantLeaves(node: KeepFieldNode): KeepFieldLeaf[] {
  return 'children' in node ? node.children.flatMap(descendantLeaves) : [node];
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
      keep: createDefaultProfileRemoteKeepFields(),
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
  const keepRemoteSource =
    keepFieldsRemoteIndex.value == null
      ? null
      : props.remotes[keepFieldsRemoteIndex.value];
  const keepRemote = keepRemoteSource
    ? {
        ...keepRemoteSource,
        keep: keepFieldsDraft.value ?? keepRemoteSource.keep,
      }
    : null;
  const leaf = (
    id: string,
    checked: boolean,
    setChecked: KeepFieldLeaf['setChecked'],
    labelKey = id.split('.').at(-1) ?? id,
  ): KeepFieldLeaf => ({
    id,
    label: t(`profiles.dialog.keepField.${labelKey}`),
    checked,
    setChecked,
  });
  const keepFieldNodes: KeepFieldNode[] = !keepRemote
    ? []
    : keepRemote.format === 'clash'
      ? [
          leaf(
            'clash.proxies',
            keepRemote.keep.clash.proxies,
            (keep, checked) => {
              keep.clash.proxies = checked;
            },
            'clash_proxies',
          ),
        ]
      : [
          leaf('outbounds', keepRemote.keep.outbounds, (keep, checked) => {
            keep.outbounds = checked;
          }),
          leaf('inbounds', keepRemote.keep.inbounds, (keep, checked) => {
            keep.inbounds = checked;
          }),
          {
            id: 'dns',
            label: t('profiles.dialog.keepSingboxDns'),
            children: [
              leaf(
                'dns.servers',
                keepRemote.keep.dns.servers,
                (keep, checked) => {
                  keep.dns.servers = checked;
                },
              ),
              leaf('dns.rules', keepRemote.keep.dns.rules, (keep, checked) => {
                keep.dns.rules = checked;
              }),
              leaf(
                'dns.final',
                keepRemote.keep.dns.final,
                (keep, checked) => {
                  keep.dns.final = checked;
                },
                'dns_final',
              ),
              leaf(
                'dns.strategy',
                keepRemote.keep.dns.strategy,
                (keep, checked) => {
                  keep.dns.strategy = checked;
                },
              ),
              leaf(
                'dns.disable_cache',
                keepRemote.keep.dns.disable_cache,
                (keep, checked) => {
                  keep.dns.disable_cache = checked;
                },
              ),
              leaf(
                'dns.disable_expire',
                keepRemote.keep.dns.disable_expire,
                (keep, checked) => {
                  keep.dns.disable_expire = checked;
                },
              ),
              leaf(
                'dns.independent_cache',
                keepRemote.keep.dns.independent_cache,
                (keep, checked) => {
                  keep.dns.independent_cache = checked;
                },
              ),
              leaf(
                'dns.cache_capacity',
                keepRemote.keep.dns.cache_capacity,
                (keep, checked) => {
                  keep.dns.cache_capacity = checked;
                },
              ),
              {
                id: 'dns-optimistic',
                label: t('profiles.dialog.keepField.optimistic'),
                children: [
                  leaf(
                    'dns.optimistic.enabled',
                    keepRemote.keep.dns.optimistic.enabled,
                    (keep, checked) => {
                      keep.dns.optimistic.enabled = checked;
                    },
                  ),
                  leaf(
                    'dns.optimistic.timeout',
                    keepRemote.keep.dns.optimistic.timeout,
                    (keep, checked) => {
                      keep.dns.optimistic.timeout = checked;
                    },
                  ),
                ],
              },
              leaf(
                'dns.timeout',
                keepRemote.keep.dns.timeout,
                (keep, checked) => {
                  keep.dns.timeout = checked;
                },
              ),
              leaf(
                'dns.reverse_mapping',
                keepRemote.keep.dns.reverse_mapping,
                (keep, checked) => {
                  keep.dns.reverse_mapping = checked;
                },
              ),
              leaf(
                'dns.client_subnet',
                keepRemote.keep.dns.client_subnet,
                (keep, checked) => {
                  keep.dns.client_subnet = checked;
                },
              ),
              leaf(
                'dns.fakeip',
                keepRemote.keep.dns.fakeip,
                (keep, checked) => {
                  keep.dns.fakeip = checked;
                },
              ),
            ],
          },
          {
            id: 'singbox-route',
            label: t('profiles.dialog.keepParentSingboxRoute'),
            children: [
              leaf(
                'route.rules',
                keepRemote.keep.route.rules,
                (keep, checked) => {
                  keep.route.rules = checked;
                },
              ),
              leaf(
                'route.rule_set',
                keepRemote.keep.route.rule_set,
                (keep, checked) => {
                  keep.route.rule_set = checked;
                },
              ),
              leaf(
                'route.final',
                keepRemote.keep.route.final,
                (keep, checked) => {
                  keep.route.final = checked;
                },
                'route_final',
              ),
              ...(
                [
                  'auto_detect_interface',
                  'override_android_vpn',
                  'default_interface',
                  'default_mark',
                  'find_process',
                  'find_neighbor',
                  'dhcp_lease_files',
                  'default_http_client',
                  'default_domain_resolver',
                  'default_network_strategy',
                  'default_network_type',
                  'default_fallback_network_type',
                  'default_fallback_delay',
                ] as const
              ).map((key) =>
                leaf(
                  `route.${key}`,
                  keepRemote.keep.route[key],
                  (keep, checked) => {
                    keep.route[key] = checked;
                  },
                ),
              ),
            ],
          },
          {
            id: 'singbox-experimental',
            label: t('profiles.dialog.keepParentSingboxExperimental'),
            children: [
              {
                id: 'experimental-cache-file',
                label: t('profiles.dialog.keepSingboxCacheFile'),
                children: (
                  [
                    'enabled',
                    'path',
                    'cache_id',
                    'store_fakeip',
                    'store_rdrc',
                    'rdrc_timeout',
                    'store_dns',
                  ] as const
                ).map((key) =>
                  leaf(
                    `experimental.cache_file.${key}`,
                    keepRemote.keep.experimental.cache_file[key],
                    (keep, checked) => {
                      keep.experimental.cache_file[key] = checked;
                    },
                  ),
                ),
              },
              {
                id: 'experimental-clash-api',
                label: t('profiles.dialog.keepSingboxClashApi'),
                children: (
                  [
                    'external_controller',
                    'external_ui',
                    'external_ui_download_url',
                    'external_ui_download_detour',
                    'secret',
                    'default_mode',
                    'access_control_allow_origin',
                    'access_control_allow_private_network',
                    'store_mode',
                    'store_selected',
                    'store_fakeip',
                    'cache_file',
                    'cache_id',
                  ] as const
                ).map((key) =>
                  leaf(
                    `experimental.clash_api.${key}`,
                    keepRemote.keep.experimental.clash_api[key],
                    (keep, checked) => {
                      keep.experimental.clash_api[key] = checked;
                    },
                  ),
                ),
              },
              leaf(
                'experimental.v2ray_api',
                keepRemote.keep.experimental.v2ray_api,
                (keep, checked) => {
                  keep.experimental.v2ray_api = checked;
                },
              ),
            ],
          },
        ];
  function renderKeepNode(node: KeepFieldNode, nodeIndex: number): VNode {
    const remoteIndex = keepFieldsRemoteIndex.value;
    const rowClass = [
      'text-on-surface',
      nodeIndex > 0 ? 'border-t border-outline-variant/50' : '',
    ];
    if ('children' in node) {
      const leaves = descendantLeaves(node);
      const selectedCount = leaves.filter((leaf) => leaf.checked).length;
      const checked = selectedCount === leaves.length;
      const indeterminate = selectedCount > 0 && !checked;
      return (
        <section key={node.id} class={rowClass}>
          <label
            class={[
              'flex h-10 items-center gap-3 px-3',
              'bg-surface-container-low',
              'cursor-pointer hover:bg-surface-container',
            ]}
          >
            <Checkbox
              checked={checked}
              indeterminate={indeterminate}
              onChange={(nextChecked) => {
                if (remoteIndex == null) return;
                updateKeepFields(leaves, indeterminate ? true : nextChecked);
              }}
            />
            <span class="min-w-0 truncate font-mono text-sm font-semibold">
              {node.label}
            </span>
          </label>
          <div class="ml-5 border-l border-outline-variant">
            {node.children.map(renderKeepNode)}
          </div>
        </section>
      );
    }
    return (
      <label
        key={node.id}
        class={[
          'flex h-10 items-center gap-3 px-3',
          'bg-surface-container-lowest',
          ...rowClass,
          'cursor-pointer hover:bg-surface-container-low',
        ]}
      >
        <Checkbox
          checked={node.checked}
          onChange={(checked) => {
            if (remoteIndex == null) return;
            updateKeepFields([node], checked);
          }}
        />
        <span class="min-w-0 truncate text-sm font-medium">{node.label}</span>
      </label>
    );
  }
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
                        <fieldset class="min-w-0">
                          <legend class="mb-1 block text-xs font-medium text-on-surface-variant">
                            {t('profiles.dialog.remoteFormat')}
                          </legend>
                          <div class="flex h-9 items-center gap-4">
                            <label class="flex h-9 items-center gap-2 text-sm text-on-surface">
                              <Radio
                                checked={remote.format === 'clash'}
                                name={`remote-format-${index}`}
                                value="clash"
                                onChange={() =>
                                  updateRemote(index, { format: 'clash' })
                                }
                              />
                              {t('profiles.dialog.remoteFormatClash')}
                            </label>
                            <label class="flex h-9 items-center gap-2 text-sm text-on-surface">
                              <Radio
                                checked={remote.format === 'singbox'}
                                name={`remote-format-${index}`}
                                value="singbox"
                                onChange={() =>
                                  updateRemote(index, { format: 'singbox' })
                                }
                              />
                              {t('profiles.dialog.remoteFormatSingbox')}
                            </label>
                          </div>
                        </fieldset>
                        <div>
                          <span class="mb-1 block text-xs font-medium text-on-surface-variant">
                            {t('profiles.dialog.keepFields')}
                          </span>
                          <Button
                            shape="rect"
                            size="sm"
                            variant="outline"
                            onClick={() => {
                              openKeepFields(index);
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
        contentClass="max-w-lg"
        onClose={() => {
          closeKeepFields();
        }}
      >
        {keepRemote != null ? (
          <div class="flex max-h-[80vh] min-h-0 flex-col overflow-hidden rounded border border-outline-variant">
            <div class="min-h-0 flex-1 overflow-y-auto">
              {keepFieldNodes.map(renderKeepNode)}
            </div>
            <div class="flex h-14 shrink-0 items-center justify-end gap-2 border-t border-outline-variant bg-surface-container-lowest px-3">
              <Button
                shape="rect"
                size="sm"
                variant="ghost"
                onClick={closeKeepFields}
              >
                {t('common.cancel')}
              </Button>
              <Button shape="rect" size="sm" onClick={saveKeepFields}>
                {t('common.save')}
              </Button>
            </div>
          </div>
        ) : null}
      </Dialog>
    </>
  );
});
