import { __render } from '@/shared/helpter';
import { clearScoreLogs, getScoreLogHistory } from '@/api/score';
import { useClientQuery } from '@/hooks/use-client-query';
import { useScoreStreamData } from '@/hooks/use-score-stream';
import { Button, IconButton } from '@/components/button';
import { toast } from '@/components/toast';
import { Divider } from '@/components/divider';
import { Icon } from '@/components/icon';
import { Page, PageContent } from '@/components/page-content';
import { CloseOutlined, SearchOutlined } from '@vicons/material';
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useMountedOrActivated } from '@/hooks/use-mounted-or-activated';
import { useUnmountedOrDeactivated } from '@/hooks/use-unmounted-or-deactivated';
import { useI18n } from 'vue-i18n';
import { i18n } from '@/i18n';

interface KernelLogEntry {
  id: number;
  level: string;
  timestamp: string;
  message: string;
}

const MAX_LOG_ENTRIES = 1_000;

const entries = ref<KernelLogEntry[]>([]);
const searchKeyword = ref('');
const selectedLevels = ref(['error', 'warn', 'info', 'debug']);
const sortDirection = ref<'asc' | 'desc'>('desc');
const { connectionState, subscribeLogs } = useScoreStreamData();
useClientQuery({
  queryKey: ['scoreLogHistory'],
  queryFn: async () => {
    const history = await getScoreLogHistory();
    entries.value = history.slice(-MAX_LOG_ENTRIES).map(createEntry);
  },
});
const clearLogsQuery = useClientQuery({
  queryKey: ['clearScoreLogs'],
  queryFn: async () => {
    try {
      await clearScoreLogs();
      entries.value = [];
    } catch (error) {
      toast.error({
        title:
          error instanceof Error
            ? error.message
            : i18n.global.t('errors.clearLogs'),
      });
    }
  },
  enabled: false,
});
let nextLogId = 0;
let unsubscribeLogs: (() => void) | undefined;

const logLevels = ['error', 'warn', 'info', 'debug'];

const filteredEntries = computed(() => {
  const keyword = searchKeyword.value.trim().toLowerCase();
  const matched = entries.value.filter(
    (entry) =>
      selectedLevels.value.includes(entry.level) &&
      (!keyword ||
        `${entry.level} ${entry.message}`.toLowerCase().includes(keyword)),
  );
  return [...matched].sort((left, right) => {
    const comparison =
      left.timestamp.localeCompare(right.timestamp) || left.id - right.id;
    return sortDirection.value === 'asc' ? comparison : -comparison;
  });
});

function addEntry(rawMessage: unknown) {
  const entry = createEntry(rawMessage);
  entries.value = [...entries.value.slice(-(MAX_LOG_ENTRIES - 1)), entry];
}

function createEntry(rawMessage: unknown): KernelLogEntry {
  let level = 'info';
  let message =
    typeof rawMessage === 'string' ? rawMessage : JSON.stringify(rawMessage);
  let payload: { payload?: unknown; type?: unknown } | undefined;
  if (typeof rawMessage === 'string') {
    try {
      payload = JSON.parse(rawMessage) as typeof payload;
    } catch {
      // Controller may send plain-text log entries.
    }
  } else if (rawMessage && typeof rawMessage === 'object') {
    payload = rawMessage as typeof payload;
  }
  if (payload) {
    if (typeof payload.type === 'string') {
      level = payload.type.toLowerCase();
    }
    if (typeof payload.payload === 'string') {
      message = payload.payload;
    }
  }

  message = message.replace(/\x1b\[[0-9;]*m/g, '');
  const timestampMatch = message.match(
    /^(?:[+-]\d{4}\s+)?(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})(?:\.(\d{1,3}))?\s+/,
  );
  const timestamp = timestampMatch
    ? `${timestampMatch[1]}${timestampMatch[2] ? `.${timestampMatch[2]}` : ''}`
    : formatTimestamp(new Date());
  if (timestampMatch) {
    message = message.slice(timestampMatch[0].length);
  }
  const normalized = message.toLowerCase();
  if (normalized.includes('error') || normalized.includes('fatal')) {
    level = 'error';
  } else if (normalized.includes('warn')) {
    level = 'warn';
  } else if (normalized.includes('debug')) {
    level = 'debug';
  }
  return { id: nextLogId++, level, timestamp, message };
}

function formatTimestamp(date: Date) {
  const pad = (value: number, size = 2) => String(value).padStart(size, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function toggleLevel(level: string) {
  selectedLevels.value = selectedLevels.value.includes(level)
    ? selectedLevels.value.filter((item) => item !== level)
    : [...selectedLevels.value, level];
}

function clearLogs() {
  void clearLogsQuery.refetch();
}

useMountedOrActivated(() => {
  unsubscribeLogs = subscribeLogs(addEntry);
});
useUnmountedOrDeactivated(() => unsubscribeLogs?.());

defineOptions({ name: 'LogsPage' });
const { t } = useI18n();

export default __render(() => {
  return (
    <Page title={t('logs.title')}>
      {{
        actions: () => (
          <>
            <div class="hidden items-center gap-1 lg:flex">
              <Button
                shape="pill"
                size="xs"
                variant={
                  selectedLevels.value.length === logLevels.length
                    ? 'solid'
                    : 'ghost'
                }
                onClick={() => {
                  selectedLevels.value = [...logLevels];
                }}
              >
                {t('logs.all')}
              </Button>
              {logLevels.map((level) => (
                <Button
                  key={level}
                  shape="pill"
                  size="xs"
                  variant={
                    selectedLevels.value.includes(level) ? 'solid' : 'ghost'
                  }
                  onClick={() => {
                    toggleLevel(level);
                  }}
                >
                  {level}
                </Button>
              ))}
              <Divider class="mx-2" orientation="vertical" />
              <Button
                class="w-18"
                shape="pill"
                size="xs"
                variant="outline"
                onClick={() => {
                  sortDirection.value =
                    sortDirection.value === 'desc' ? 'asc' : 'desc';
                }}
              >
                {sortDirection.value === 'desc'
                  ? t('logs.newest')
                  : t('logs.oldest')}
              </Button>
            </div>
            <label class="hidden h-8 w-64 items-center gap-2 rounded border border-outline-variant bg-surface px-3 text-on-surface-variant lg:flex">
              <Icon class="text-lg">
                <SearchOutlined />
              </Icon>
              <input
                class="min-w-0 flex-1 bg-transparent text-sm text-on-surface outline-none placeholder:text-on-surface-variant"
                value={searchKeyword.value}
                onInput={(event) => {
                  searchKeyword.value = (
                    event.target as HTMLInputElement
                  ).value;
                }}
                placeholder={t('logs.search')}
              />
            </label>
            <IconButton
              disabled={clearLogsQuery.isFetching.value}
              title={t('logs.clear')}
              onClick={clearLogs}
            >
              <CloseOutlined />
            </IconButton>
          </>
        ),
        default: () => (
          <PageContent class="pb-24 pt-4">
            <section class="rounded border border-outline-variant bg-surface-container-lowest">
              <div class="rounded bg-surface p-3 font-mono text-xs leading-5">
                {filteredEntries.value.length ? (
                  filteredEntries.value.map((entry) => (
                    <div
                      key={entry.id}
                      class="flex gap-3 border-b border-outline-variant/30 py-1.5 last:border-b-0"
                    >
                      <time class="w-44 shrink-0 text-on-surface-variant">
                        {entry.timestamp}
                      </time>
                      <span
                        class={[
                          'w-12 shrink-0 font-semibold uppercase',
                          entry.level.includes('error')
                            ? 'text-error'
                            : entry.level.includes('warn')
                              ? 'text-warn'
                              : 'text-secondary',
                        ]}
                      >
                        {entry.level}
                      </span>
                      <span class="min-w-0 select-text wrap-break-word text-on-surface">
                        {entry.message}
                      </span>
                    </div>
                  ))
                ) : (
                  <div class="flex min-h-32 items-center justify-center text-sm text-on-surface-variant">
                    {connectionState.value === 'connected'
                      ? t('logs.waiting')
                      : t('logs.connectToView')}
                  </div>
                )}
              </div>
            </section>
          </PageContent>
        ),
      }}
    </Page>
  );
});
