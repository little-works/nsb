import {
  createTemplate,
  deleteTemplate,
  getDefaultTemplate,
  updateTemplate,
} from '@/api/client';
import { IconButton } from '@/components/button';
import { Input } from '@/components/input';
import { Page, PageContent } from '@/components/page-content';
import { toast } from '@/components/toast';
import TemplateJsonEditor from '@/pages/profiles/profile-node-editor.setup';
import { __render } from '@/shared/helpter';
import { useTemplates } from '@/store/app';
import type { ProfileTemplate } from '@/types';
import {
  AddOutlined,
  DeleteOutlineOutlined,
  EditOutlined,
} from '@vicons/material';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { i18n } from '@/i18n';

defineOptions({ name: 'TemplatesPage' });

const templatesQuery = useTemplates();
const editingTemplate = ref<ProfileTemplate | null>(null);
const editorOpen = ref(false);
const name = ref('');
const content = ref('{}');
const saving = ref(false);
const creating = ref(false);
const templates = computed(() => templatesQuery.data.value ?? []);
const loading = computed(() => templatesQuery.isFetching.value);

function formatTimestamp(timestamp: number) {
  if (!timestamp) return '–';
  return new Intl.DateTimeFormat(i18n.global.locale.value, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(new Date(timestamp * 1000));
}

async function startCreate() {
  if (creating.value) return;
  creating.value = true;
  try {
    editingTemplate.value = null;
    name.value = t('templates.defaultName');
    content.value = await getDefaultTemplate();
    editorOpen.value = true;
  } catch (error) {
    toast.error({
      title:
        error instanceof Error
          ? error.message
          : i18n.global.t('templates.createFailed'),
    });
  } finally {
    creating.value = false;
  }
}

function startEdit(template: ProfileTemplate) {
  editingTemplate.value = template;
  name.value = template.name;
  content.value = template.content;
  editorOpen.value = true;
}

function closeEditor() {
  if (saving.value) return;
  editorOpen.value = false;
  editingTemplate.value = null;
}

async function saveTemplate() {
  if (!name.value.trim()) {
    toast.error({ title: i18n.global.t('templates.nameRequired') });
    return;
  }
  saving.value = true;
  try {
    const payload = { name: name.value.trim(), content: content.value };
    const created = !editingTemplate.value;
    if (editingTemplate.value) {
      await updateTemplate(editingTemplate.value.id, payload);
    } else {
      await createTemplate(payload);
    }
    await templatesQuery.refetch();
    toast.info({
      title: i18n.global.t(created ? 'templates.created' : 'templates.updated'),
    });
    editorOpen.value = false;
    editingTemplate.value = null;
  } catch (error) {
    toast.error({
      title:
        error instanceof Error
          ? error.message
          : i18n.global.t('templates.saveFailed'),
    });
  } finally {
    saving.value = false;
  }
}

async function removeTemplate(template: ProfileTemplate) {
  if (
    !window.confirm(
      i18n.global.t('templates.confirmDelete', { name: template.name }),
    )
  ) {
    return;
  }
  try {
    await deleteTemplate(template.id);
    await templatesQuery.refetch();
    toast.info({
      title: i18n.global.t('templates.deleted', { name: template.name }),
    });
  } catch (error) {
    toast.error({
      title:
        error instanceof Error
          ? error.message
          : i18n.global.t('templates.deleteFailed'),
    });
  }
}

const { t } = useI18n();

export default __render(() => (
  <Page title={t('templates.title')} subtitle={t('templates.subtitle')}>
    {{
      actions: () => (
        <IconButton
          aria-label={t('templates.add')}
          disabled={creating.value}
          size="sm"
          tooltip={t('templates.add')}
          onClick={() => void startCreate()}
        >
          <AddOutlined />
        </IconButton>
      ),
      default: () => (
        <PageContent class="pb-24 pt-4">
          {loading.value && !templates.value.length ? (
            <div class="rounded border border-outline-variant bg-surface-container-lowest px-4 py-8 text-center text-sm text-on-surface-variant">
              {t('common.loading')}
            </div>
          ) : templates.value.length ? (
            <section class="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
              {templates.value.map((template) => (
                <article
                  key={template.id}
                  class={[
                    'flex min-w-0 flex-col rounded border border-outline-variant p-3 shadow-sm',
                    'bg-surface-container-lowest transition-shadow hover:shadow-md',
                  ]}
                >
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <h2 class="truncate text-sm font-semibold text-on-surface">
                        {template.name}
                      </h2>
                      <p class="mt-1 text-xs text-on-surface-variant">
                        {t('templates.references', {
                          count: template.reference_count,
                        })}
                      </p>
                    </div>
                    <div class="flex shrink-0 items-center gap-1">
                      <IconButton
                        aria-label={t('common.edit')}
                        size="xs"
                        tooltip={t('common.edit')}
                        onClick={() => startEdit(template)}
                      >
                        <EditOutlined />
                      </IconButton>
                      <IconButton
                        aria-label={t('common.delete')}
                        size="xs"
                        tooltip={t('common.delete')}
                        onClick={() => void removeTemplate(template)}
                      >
                        <DeleteOutlineOutlined />
                      </IconButton>
                    </div>
                  </div>
                  <p class="mt-5 text-xs text-on-surface-variant">
                    {t('templates.updatedAt', {
                      time: formatTimestamp(template.updated_at),
                    })}
                  </p>
                </article>
              ))}
            </section>
          ) : (
            <div class="rounded border border-outline-variant bg-surface-container-lowest px-4 py-8 text-center text-sm text-on-surface-variant">
              {t('templates.empty')}
            </div>
          )}
        </PageContent>
      ),
      overlay: () => (
        <TemplateJsonEditor
          content={content.value}
          description={t('templates.editorDescription')}
          loading={creating.value}
          open={editorOpen.value}
          saveLabel={t('common.save')}
          saving={saving.value}
          title={
            editingTemplate.value
              ? t('templates.editTitle')
              : t('templates.createTitle')
          }
          onClose={closeEditor}
          onContentChange={(value) => {
            content.value = value;
          }}
          onSave={() => void saveTemplate()}
          v-slots={{
            form: () => (
              <label class="mb-3 block">
                <span class="mb-1 block text-sm font-medium text-on-surface">
                  {t('templates.name')}
                </span>
                <Input
                  value={name.value}
                  placeholder={t('templates.namePlaceholder')}
                  onInput={(event) => {
                    name.value = (event.target as HTMLInputElement).value;
                  }}
                />
              </label>
            ),
          }}
        />
      ),
    }}
  </Page>
));
