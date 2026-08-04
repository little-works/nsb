import { __render } from '@/shared/helpter';
import { Button, IconButton } from '@/components/button';
import { Dropdown, type DropdownSlotProps } from '@/components/dropdown';
import { Icon } from '@/components/icon';
import { Tooltip } from '@/components/tooltip';
import {
  CancelOutlined,
  CheckCircleOutlined,
  MoreVertOutlined,
} from '@vicons/material';
import type { ProfileSummary } from '@/types';
import { useI18n } from 'vue-i18n';
import { i18n } from '@/i18n';

export interface ProfilesTableProps {
  profiles: ProfileSummary[];
  currentProfileId: string | null;
  loading: boolean;
  onEdit: (item: ProfileSummary) => void;
  onDelete: (item: ProfileSummary) => void;
  onEditNodes: (item: ProfileSummary) => void;
  onSetCurrent: (item: ProfileSummary) => void | Promise<void>;
  onRefresh: (item: ProfileSummary) => void | Promise<void>;
}

const props = defineProps<ProfilesTableProps>();

function formatTimestamp(timestamp: number, full = false) {
  if (!timestamp) return i18n.global.t('profiles.neverUpdated');

  const date = new Date(timestamp * 1000);
  const now = new Date();
  const oneYearAgo = new Date(now);
  oneYearAgo.setFullYear(now.getFullYear() - 1);
  const moreThanSevenDaysAgo =
    now.getTime() - date.getTime() > 7 * 24 * 60 * 60 * 1000;

  return new Intl.DateTimeFormat(i18n.global.locale.value, {
    year: full || date < oneYearAgo ? 'numeric' : undefined,
    month: '2-digit',
    day: '2-digit',
    hour: full || !moreThanSevenDaysAgo ? '2-digit' : undefined,
    minute: full || !moreThanSevenDaysAgo ? '2-digit' : undefined,
    hour12: false,
  }).format(date);
}

defineOptions({ name: 'ProfilesTable' });
const { t } = useI18n();

export default __render<ProfilesTableProps>(() => {
  return (
    <section>
      {props.loading && props.profiles.length === 0 ? (
        <div class="rounded border border-outline-variant bg-surface-container-lowest px-4 py-8 text-center text-sm text-on-surface-variant">
          {t('profiles.loading')}
        </div>
      ) : props.profiles.length === 0 ? (
        <div class="rounded border border-outline-variant bg-surface-container-lowest px-4 py-8 text-center text-sm text-on-surface-variant">
          {t('profiles.empty')}
        </div>
      ) : (
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {props.profiles.map((item) => {
            const isCurrent = item.id === props.currentProfileId;
            const hasAttempt = Boolean(item.last_attempt_at);
            const updateFailed = hasAttempt && item.last_update_failed;
            return (
              <article
                key={item.id}
                class="flex min-w-0 flex-col
                  rounded border border-outline-variant bg-surface-container-lowest p-3 shadow-sm
                  transition-shadow hover:shadow-md"
              >
                <div class="mb-2 flex items-center justify-between gap-3">
                  <div class="flex min-w-0 items-center gap-3">
                    <span
                      class={[
                        'h-2 w-2 shrink-0 rounded-full',
                        isCurrent ? 'bg-secondary' : 'bg-outline-variant',
                      ]}
                    />
                    <span class="truncate text-sm font-semibold text-on-surface">
                      {item.name}
                    </span>
                  </div>
                  <div class="flex shrink-0 items-center gap-1">
                    <Button
                      shape="rect"
                      size="xs"
                      variant="ghost"
                      onClick={() => props.onEdit(item)}
                    >
                      {t('common.edit')}
                    </Button>
                    <Button
                      shape="rect"
                      size="xs"
                      variant="primary-ghost"
                      onClick={() => props.onRefresh(item)}
                    >
                      {t('common.update')}
                    </Button>
                    <Dropdown
                      placement="bottom-end"
                      v-slots={{
                        default: ({ open }: DropdownSlotProps) => (
                          <IconButton
                            aria-label={t('profiles.moreActions', {
                              name: item.name,
                            })}
                            aria-expanded={open}
                            size="xs"
                            title={t('profiles.moreActions', {
                              name: item.name,
                            })}
                          >
                            <MoreVertOutlined />
                          </IconButton>
                        ),
                        overlay: ({ close }: DropdownSlotProps) => (
                          <>
                            <Button
                              block
                              class="px-2"
                              shape="rect"
                              size="xs"
                              variant="ghost"
                              onClick={() => {
                                close();
                                props.onEditNodes(item);
                              }}
                            >
                              {t('profiles.editNodes')}
                            </Button>
                            <div class="my-1 h-px bg-outline-variant" />
                            <Button
                              block
                              class="px-2"
                              shape="rect"
                              size="xs"
                              variant="danger-ghost"
                              onClick={() => {
                                close();
                                props.onDelete(item);
                              }}
                            >
                              {t('profiles.delete')}
                            </Button>
                          </>
                        ),
                      }}
                    />
                  </div>
                </div>
                <p
                  class="mb-2 truncate rounded bg-surface-container-high px-2 py-1 font-mono text-xs text-outline"
                  title={item.kind}
                >
                  {item.kind}
                </p>
                <div class="mt-auto flex items-center justify-between gap-3">
                  <div
                    class="flex min-w-0 items-center gap-1.5
                      text-xs text-on-surface-variant"
                  >
                    {hasAttempt ? (
                      updateFailed ? (
                        <span
                          aria-label={t('profiles.lastAttemptFailed')}
                          class="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-xs outline-none
                            focus-visible:ring-2 focus-visible:ring-error"
                          tabindex={0}
                        >
                          <Icon class="text-xs text-error">
                            <CancelOutlined />
                          </Icon>
                        </span>
                      ) : (
                        <Icon
                          aria-label={t('profiles.lastAttemptSucceeded')}
                          class="text-xs text-secondary"
                        >
                          <CheckCircleOutlined />
                        </Icon>
                      )
                    ) : null}
                    {hasAttempt ? (
                      <Tooltip
                        content={formatTimestamp(item.last_attempt_at, true)}
                      >
                        <span
                          class="truncate rounded-xs outline-none
                            focus-visible:ring-2 focus-visible:ring-secondary"
                          tabindex={0}
                        >
                          {formatTimestamp(item.last_attempt_at)}
                        </span>
                      </Tooltip>
                    ) : (
                      <span class="truncate">{t('profiles.neverUpdated')}</span>
                    )}
                  </div>
                  <Button
                    class="px-1 font-medium hover:underline"
                    disabled={isCurrent}
                    shape="rect"
                    size="xs"
                    variant="primary-ghost"
                    onClick={() => props.onSetCurrent(item)}
                  >
                    {t(isCurrent ? 'profiles.inUse' : 'profiles.use')}
                  </Button>
                </div>
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
});
