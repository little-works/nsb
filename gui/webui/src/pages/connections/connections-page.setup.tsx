import { __render } from '@/shared/helpter';
import { useScoreStreamData } from '@/hooks/use-score-stream';
import { Icon } from '@/components/icon';
import { Page, PageContent } from '@/components/page-content';
import type { CoreApiConnectionsData } from '@/types';
import { SearchOutlined } from '@vicons/material';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';

type Connection = CoreApiConnectionsData['connections'][number];

const { connections: streamConnections, connectionState } =
  useScoreStreamData();
const connections = computed<Connection[]>(() =>
  (streamConnections.value?.connections ?? []).map((connection) => ({
    ...connection,
    chains: [...connection.chains],
    metadata: { ...connection.metadata },
  })),
);
const memory = computed(() => streamConnections.value?.memory ?? 0);
const uploadTotal = computed(() => streamConnections.value?.uploadTotal ?? 0);
const downloadTotal = computed(
  () => streamConnections.value?.downloadTotal ?? 0,
);
const loading = computed(() => connectionState.value === 'connecting');
const searchKeyword = ref('');

const filteredConnections = computed<Connection[]>(() => {
  const keyword = searchKeyword.value.trim().toLowerCase();
  if (!keyword) {
    return connections.value;
  }
  return connections.value.filter((connection) =>
    [
      connection.metadata.host,
      connection.metadata.destinationIP,
      connection.metadata.processPath,
      connection.metadata.network,
      connection.rule,
      connection.chains.join(' '),
    ]
      .join(' ')
      .toLowerCase()
      .includes(keyword),
  );
});

function formatBytes(bytes: number, locale: string) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return '0 B';
  }
  const units = ['B', 'KB', 'MB', 'GB'];
  const index = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1,
  );
  const value = bytes / 1024 ** index;
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: value >= 10 || index === 0 ? 0 : 1 }).format(value)} ${units[index]}`;
}

function destination(connection: Connection, unknownDestination: string) {
  const { host, destinationIP, destinationPort } = connection.metadata;
  const target = host || destinationIP || unknownDestination;
  return destinationPort ? `${target}:${destinationPort}` : target;
}

function source(connection: Connection) {
  const { sourceIP, sourcePort } = connection.metadata;
  return sourcePort ? `${sourceIP}:${sourcePort}` : sourceIP || '-';
}

function connectedFor(start: string, locale: string) {
  const startedAt = Date.parse(start);
  if (Number.isNaN(startedAt)) {
    return '-';
  }
  const seconds = Math.max(0, Math.floor((Date.now() - startedAt) / 1000));
  const minutes = Math.floor(seconds / 60);
  const formatter = new Intl.NumberFormat(locale);
  return minutes
    ? `${formatter.format(minutes)}m ${formatter.format(seconds % 60)}s`
    : `${formatter.format(seconds)}s`;
}

defineOptions({ name: 'ConnectionsPage' });
const { t, locale } = useI18n();

export default __render(() => {
  return (
    <Page title={t('connections.title')} subtitle={t('connections.subtitle')}>
      {{
        actions: () => (
          <>
            <label class="hidden h-8 w-64 items-center gap-2 rounded border border-outline-variant bg-surface px-3 text-on-surface-variant lg:flex">
              <Icon class="text-lg">
                <SearchOutlined />
              </Icon>
              <input
                class="min-w-0 flex-1 bg-transparent text-sm text-on-surface outline-none placeholder:text-on-surface-variant"
                placeholder={t('connections.search')}
                value={searchKeyword.value}
                onInput={(event) => {
                  searchKeyword.value = (
                    event.target as HTMLInputElement
                  ).value;
                }}
              />
            </label>
          </>
        ),
        default: () => (
          <PageContent class="pb-24">
            <div class="-mx-4 flex items-center bg-background px-4 py-4 sm:sticky sm:top-0 sm:z-page-header sm:h-28 sm:py-0">
              <section class="grid w-full grid-cols-1 gap-3 sm:grid-cols-3">
                <div class="rounded border border-outline-variant bg-surface-container-lowest p-4">
                  <p class="text-xs font-medium tracking-wide text-on-surface-variant">
                    {t('connections.active')}
                  </p>
                  <p class="mt-1 text-2xl font-bold text-primary">
                    {connections.value.length}
                  </p>
                </div>
                <div class="rounded border border-outline-variant bg-surface-container-lowest p-4">
                  <p class="text-xs font-medium tracking-wide text-on-surface-variant">
                    {t('connections.transferred')}
                  </p>
                  <p class="mt-1 text-sm font-semibold text-on-surface">
                    ↑ {formatBytes(uploadTotal.value, locale.value)} · ↓{' '}
                    {formatBytes(downloadTotal.value, locale.value)}
                  </p>
                </div>
                <div class="rounded border border-outline-variant bg-surface-container-lowest p-4">
                  <p class="text-xs font-medium tracking-wide text-on-surface-variant">
                    {t('connections.memory')}
                  </p>
                  <p class="mt-1 text-sm font-semibold text-on-surface">
                    {formatBytes(memory.value, locale.value)}
                  </p>
                </div>
              </section>
            </div>
            <section class="rounded overflow-hidden border border-outline-variant bg-surface-container-lowest">
              <div class="overflow-x-auto">
                <table class="min-w-full border-separate border-spacing-0 text-left text-sm">
                  <thead class="z-sticky bg-background text-xs  tracking-wide text-on-surface-variant shadow-sm">
                    <tr class="bg-surface-container-low">
                      <th class="rounded-tl px-4 py-3 font-medium">
                        {t('connections.destination')}
                      </th>
                      <th class="px-4 py-3 font-medium">
                        {t('connections.source')}
                      </th>
                      <th class="px-4 py-3 font-medium">
                        {t('connections.network')}
                      </th>
                      <th class="px-4 py-3 font-medium">
                        {t('connections.chain')}
                      </th>
                      <th class="px-4 py-3 font-medium">
                        {t('connections.rule')}
                      </th>
                      <th class="px-4 py-3 font-medium">
                        {t('connections.transfer')}
                      </th>
                      <th class="rounded-tr px-4 py-3 font-medium">
                        {t('connections.duration')}
                      </th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-outline-variant/50">
                    {filteredConnections.value.map((connection) => (
                      <tr
                        key={connection.id}
                        class="hover:bg-surface-container-low"
                      >
                        <td class="max-w-64 px-4 py-3 font-medium text-on-surface">
                          <div class="truncate">
                            {destination(
                              connection,
                              t('connections.unknownDestination'),
                            )}
                          </div>
                          <div class="truncate text-xs font-normal text-on-surface-variant">
                            {connection.metadata.processPath ||
                              connection.metadata.type}
                          </div>
                        </td>
                        <td class="px-4 py-3 font-mono text-xs text-on-surface-variant">
                          {source(connection)}
                        </td>
                        <td class="px-4 py-3 text-on-surface-variant">
                          {connection.metadata.network || '-'}
                        </td>
                        <td class="max-w-48 px-4 py-3 text-on-surface">
                          {connection.chains.join(' → ') || '-'}
                        </td>
                        <td class="max-w-48 px-4 py-3 text-on-surface-variant">
                          {connection.rule || connection.rulePayload || '-'}
                        </td>
                        <td class="whitespace-nowrap px-4 py-3 text-on-surface-variant">
                          ↑ {formatBytes(connection.upload, locale.value)}
                          <br />↓{' '}
                          {formatBytes(connection.download, locale.value)}
                        </td>
                        <td class="whitespace-nowrap px-4 py-3 text-on-surface-variant">
                          {connectedFor(connection.start, locale.value)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              {!filteredConnections.value.length ? (
                <div class="flex min-h-32 items-center justify-center text-sm text-on-surface-variant">
                  {loading.value
                    ? t('connections.loading')
                    : t('connections.empty')}
                </div>
              ) : null}
            </section>
          </PageContent>
        ),
      }}
    </Page>
  );
});
