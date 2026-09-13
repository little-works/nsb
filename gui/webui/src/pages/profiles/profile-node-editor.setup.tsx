import { __render } from '@/shared/helper';
import { Button } from '@/components/button';
import { CodeEditor } from '@/components/code-editor';
import { Dialog } from '@/components/dialog';
import { computed, ref, shallowRef, useSlots, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { i18n } from '@/i18n';

type JsonValue =
  boolean | null | number | string | JsonValue[] | { [key: string]: JsonValue };
type JsonPathSegment = number | string;

interface NavigationEntry {
  label: string;
  path: JsonPathSegment[];
}

export interface ProfileNodeEditorProps {
  open: boolean;
  title: string;
  description?: string;
  readOnly?: boolean;
  saveLabel?: string;
  content: string;
  loading: boolean;
  saving?: boolean;
  onClose: () => void;
  onContentChange?: (content: string) => void;
  onSave?: () => void;
}

const props = withDefaults(defineProps<ProfileNodeEditorProps>(), {
  readOnly: false,
  saveLabel: '',
  saving: false,
  onContentChange: () => {},
  onSave: () => {},
});
const slots = useSlots();
const rootValue = shallowRef<JsonValue | undefined>(undefined);
const selectedPath = ref<JsonPathSegment[]>([]);
const editorValue = ref('');
const parseError = ref('');
let emittedContent: string | undefined;

const hasParseError = computed(() => Boolean(parseError.value));
const currentValue = computed(() =>
  rootValue.value === undefined
    ? undefined
    : getValueAtPath(rootValue.value, selectedPath.value),
);
const navigationEntries = computed<NavigationEntry[]>(() => {
  const value = currentValue.value;
  if (Array.isArray(value)) {
    return value.map((item, index) => ({
      label: formatPathSegment(index, [...selectedPath.value, index]),
      path: [...selectedPath.value, index],
    }));
  }
  if (isJsonObject(value)) {
    return Object.keys(value).map((key) => ({
      label: key,
      path: [...selectedPath.value, key],
    }));
  }
  return [];
});

watch(
  () => [props.open, props.content] as const,
  ([open, content], [wasOpen]) => {
    if (!open) {
      return;
    }
    if (!wasOpen || content !== emittedContent) {
      loadContent(content);
    }
  },
);

function loadContent(content: string) {
  emittedContent = undefined;
  selectedPath.value = [];
  editorValue.value = content;

  try {
    rootValue.value = JSON.parse(content) as JsonValue;
    parseError.value = '';
  } catch (error) {
    rootValue.value = undefined;
    parseError.value = jsonErrorMessage(error);
  }
}

function selectPath(path: JsonPathSegment[]) {
  if (hasParseError.value || rootValue.value === undefined) {
    return;
  }

  selectedPath.value = path;
  editorValue.value = formatJson(getValueAtPath(rootValue.value, path));
  parseError.value = '';
}

function updateEditorValue(value: string) {
  if (props.readOnly) return;

  editorValue.value = value;

  try {
    const nextValue = JSON.parse(value) as JsonValue;
    const nextRoot =
      rootValue.value === undefined
        ? nextValue
        : setValueAtPath(rootValue.value, selectedPath.value, nextValue);
    const serialized = formatJson(nextRoot);

    rootValue.value = nextRoot;
    parseError.value = '';
    emittedContent = serialized;
    props.onContentChange?.(serialized);
  } catch (error) {
    parseError.value = jsonErrorMessage(error);
  }
}

function canSave() {
  return (
    !props.readOnly && !props.loading && !props.saving && !hasParseError.value
  );
}

function getValueAtPath(value: JsonValue, path: JsonPathSegment[]) {
  return path.reduce<JsonValue>((current, segment) => {
    if (typeof segment === 'number' && Array.isArray(current)) {
      return current[segment];
    }
    if (typeof segment === 'string' && isJsonObject(current)) {
      return current[segment];
    }
    return current;
  }, value);
}

function isJsonObject(
  value: JsonValue | undefined,
): value is Record<string, JsonValue> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function setValueAtPath(
  value: JsonValue,
  path: JsonPathSegment[],
  nextValue: JsonValue,
): JsonValue {
  if (!path.length) {
    return nextValue;
  }

  const [segment, ...rest] = path;
  if (typeof segment === 'number' && Array.isArray(value)) {
    const nextArray = [...value];
    nextArray[segment] = setValueAtPath(value[segment], rest, nextValue);
    return nextArray;
  }
  if (typeof segment === 'string' && isJsonObject(value)) {
    return {
      ...value,
      [segment]: setValueAtPath(value[segment], rest, nextValue),
    };
  }
  return value;
}

function formatJson(value: JsonValue) {
  return JSON.stringify(value, null, 2);
}

function jsonErrorMessage(error: unknown) {
  return error instanceof Error
    ? i18n.global.t('editor.invalidJson', { error: error.message })
    : i18n.global.t('editor.invalidJsonGeneric');
}

function formatPathSegment(segment: JsonPathSegment, path: JsonPathSegment[]) {
  if (typeof segment === 'number' && rootValue.value !== undefined) {
    if (path.length === 2 && path[0] === 'outbounds') {
      const outbound = getValueAtPath(rootValue.value, path);
      if (isJsonObject(outbound) && typeof outbound.tag === 'string') {
        return `[${segment}].${outbound.tag}`;
      }
    }

    if (path.length === 3 && path[0] === 'route' && path[1] === 'rules') {
      const rule = getValueAtPath(rootValue.value, path);
      if (isJsonObject(rule) && typeof rule.outbound === 'string') {
        return `[${segment}].${rule.outbound}`;
      }
    }
  }

  return typeof segment === 'number' ? `[${segment}]` : segment;
}

defineOptions({ name: 'ProfileNodeEditor' });
const { t } = useI18n();

export default __render<ProfileNodeEditorProps>(() => {
  return (
    <Dialog
      closeDisabled={props.saving}
      contentClass="flex h-[70vh] max-w-6xl flex-col"
      description={props.description ?? ''}
      open={props.open}
      title={props.title}
      onClose={props.onClose}
    >
      {props.open ? (
        <>
          {slots.form?.()}
          <div class="min-h-0 flex flex-1 overflow-hidden rounded border border-outline-variant">
            <aside class="flex w-48 shrink-0 flex-col border-r border-outline-variant bg-surface-container-low">
              <nav
                class="min-h-0 flex-1 overflow-y-auto p-2"
                aria-label={t('editor.path')}
              >
                {!selectedPath.value.length ? (
                  <button
                    class="flex h-8 w-full items-center rounded px-2
                    bg-primary-container
                    text-left text-sm text-on-primary-container"
                    type="button"
                    onClick={() => selectPath([])}
                  >
                    {t('editor.root')}
                  </button>
                ) : null}
                {navigationEntries.value.map((entry) => (
                  <button
                    key={entry.label}
                    class="mt-1 flex h-8 w-full items-center truncate rounded px-2 text-left text-sm text-on-surface hover:bg-surface-container"
                    title={entry.label}
                    type="button"
                    onClick={() => selectPath(entry.path)}
                  >
                    {entry.label}
                  </button>
                ))}
              </nav>
            </aside>

            <div class="flex min-w-0 flex-1 flex-col">
              <div class="flex h-10 shrink-0 items-center border-b border-outline-variant px-3">
                <div class="flex min-w-0 items-center gap-1 overflow-x-auto text-xs text-on-surface-variant">
                  <button
                    class="shrink-0 text-primary hover:underline"
                    type="button"
                    onClick={() => selectPath([])}
                  >
                    {t('editor.root')}
                  </button>
                  {selectedPath.value.map((segment, index) => (
                    <button
                      key={index}
                      class="shrink-0 hover:text-primary hover:underline"
                      type="button"
                      onClick={() =>
                        selectPath(selectedPath.value.slice(0, index + 1))
                      }
                    >
                      /{' '}
                      {formatPathSegment(
                        segment,
                        selectedPath.value.slice(0, index + 1),
                      )}
                    </button>
                  ))}
                </div>
              </div>

              <div class="min-h-0 flex-1">
                {props.loading ? (
                  <div class="flex h-full items-center justify-center text-sm text-on-surface-variant">
                    {t('editor.loading')}
                  </div>
                ) : (
                  <CodeEditor
                    readOnly={props.readOnly}
                    value={editorValue.value}
                    onChange={props.readOnly ? undefined : updateEditorValue}
                  />
                )}
              </div>
            </div>
          </div>

          {hasParseError.value ? (
            <p class="mt-2 text-xs text-error" role="alert">
              {parseError.value} {t('editor.fixJson')}
            </p>
          ) : null}

          {!props.readOnly ? (
            <div class="mt-3 flex justify-end gap-2">
              <Button
                disabled={props.saving}
                shape="rect"
                size="sm"
                variant="outline"
                onClick={props.onClose}
              >
                {t('common.cancel')}
              </Button>
              <Button
                disabled={!canSave()}
                shape="rect"
                size="sm"
                variant="solid"
                onClick={props.onSave}
              >
                {props.saveLabel}
              </Button>
            </div>
          ) : null}
        </>
      ) : null}
    </Dialog>
  );
});
