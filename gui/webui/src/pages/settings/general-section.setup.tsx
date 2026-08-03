import { __render } from '@/shared/helpter';
import { Button } from '@/components/button';
import { Checkbox } from '@/components/checkbox';
import { Icon } from '@/components/icon';
import { Popover } from '@/components/popover';
import { useMounted } from '@vueuse/core';
import { HelpOutlineOutlined, TuneOutlined } from '@vicons/material';
import { useI18n } from 'vue-i18n';

export interface GeneralSectionProps {
  mixedPort: string;
  appPort: string;
  allowLan: boolean;
  autoLaunchEnabled: boolean;
  autoLaunchLoading: boolean;
  systemProxyEnabled: boolean;
  loading: boolean;
  saving: boolean;
  onMixedPortChange?: (value: string) => void;
  onMixedPortBlur?: () => void | Promise<void>;
  onAppPortChange?: (value: string) => void;
  onAppPortBlur?: () => void | Promise<void>;
  onAllowLanChange?: (value: boolean) => void | Promise<void>;
  onAutoLaunchEnabledChange?: (value: boolean) => void | Promise<void>;
  onSystemProxyEnabledChange?: (value: boolean) => void | Promise<void>;
}

const props = defineProps<GeneralSectionProps>();
const mounted = useMounted();

defineOptions({ name: 'GeneralSection' });

function renderAutoLaunchHelpButton(ariaLabel: string) {
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

export default __render<GeneralSectionProps>(() => {
  const { t } = useI18n();
  const disabled = props.loading || props.saving;
  const settings = [
    {
      key: 'allow-lan',
      title: t('settings.allowLan'),
      desc: t('settings.allowLanDesc'),
      checked: props.allowLan,
      onChange: props.onAllowLanChange,
      disabled,
    },
    {
      key: 'system-proxy',
      title: t('settings.systemProxy'),
      desc: t('settings.systemProxyDesc', { port: props.mixedPort }),
      checked: props.systemProxyEnabled,
      onChange: props.onSystemProxyEnabledChange,
      disabled,
    },
    {
      key: 'auto-launch',
      title: (
        <span class="flex items-center gap-1">
          {t('settings.launchAtLogin')}
          {mounted.value ? (
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
                  renderAutoLaunchHelpButton(t('settings.launchHelp')),
                overlay: () => (
                  <div class="space-y-3">
                    <div>
                      <p class="text-sm font-semibold text-on-surface">
                        {t('settings.launchLocation')}
                      </p>
                      <p class="mt-1 text-xs leading-5 text-on-surface-variant">
                        {t('settings.launchLocationDesc')}
                      </p>
                    </div>
                    <div class="space-y-2 text-xs leading-5 text-on-surface-variant">
                      <div>
                        <p class="font-medium text-on-surface">
                          {t('settings.windows')}
                        </p>
                        <code class="mt-1 block break-all rounded bg-surface-container px-1.5 py-1 font-mono text-on-surface">
                          {t('settings.windowsLaunchPath')}
                        </code>
                      </div>
                      <p>{t('settings.macosLaunchPath')}</p>
                      <p>{t('settings.linuxLaunchPath')}</p>
                    </div>
                  </div>
                ),
              }}
            />
          ) : (
            renderAutoLaunchHelpButton(t('settings.launchHelp'))
          )}
        </span>
      ),
      desc: t('settings.launchDesc'),
      checked: props.autoLaunchEnabled,
      onChange: props.onAutoLaunchEnabledChange,
      disabled: props.autoLaunchLoading,
    },
  ];

  return (
    <div class="mb-10">
      <div class="mb-3 flex items-center gap-2">
        <Icon class="text-xl text-primary">
          <TuneOutlined />
        </Icon>
        <h3 class="text-lg font-bold leading-6 text-on-surface">
          {t('settings.general')}
        </h3>
      </div>
      <div class="overflow-hidden rounded border border-outline-variant bg-surface-container-lowest">
        <div class="border-b border-outline-variant/50 p-4">
          <label class="flex items-center gap-4">
            <div class="min-w-0 flex-1">
              <p class="text-sm font-medium leading-5 text-on-surface">
                {t('settings.appPort')}
              </p>
              <p class="text-sm leading-5 text-on-surface-variant">
                {t('settings.appPortDesc')}
              </p>
            </div>
            <input
              class="h-9 w-20 shrink-0 rounded border border-outline-variant bg-surface px-3 text-sm text-on-surface outline-none transition-all focus:border-primary"
              disabled={disabled}
              inputmode="numeric"
              placeholder={t('settings.appPortPlaceholder')}
              value={props.appPort}
              onInput={(event) =>
                props.onAppPortChange?.(
                  (event.target as HTMLInputElement).value,
                )
              }
              onBlur={() => {
                void props.onAppPortBlur?.();
              }}
            />
          </label>
        </div>
        <div class="border-b border-outline-variant/50 p-4">
          <label class="flex items-center gap-4">
            <div class="min-w-0 flex-1">
              <p class="text-sm font-medium leading-5 text-on-surface">
                {t('settings.mixedPort')}
              </p>
              <p class="text-sm leading-5 text-on-surface-variant">
                {t('settings.mixedPortDesc')}
              </p>
            </div>
            <input
              class="h-9 w-20 shrink-0 rounded border border-outline-variant bg-surface px-3 text-sm text-on-surface outline-none transition-all focus:border-primary"
              disabled={disabled}
              inputmode="numeric"
              placeholder={t('settings.mixedPortPlaceholder')}
              value={props.mixedPort}
              onInput={(event) =>
                props.onMixedPortChange?.(
                  (event.target as HTMLInputElement).value,
                )
              }
              onBlur={() => {
                void props.onMixedPortBlur?.();
              }}
            />
          </label>
        </div>
        {settings.map((item, index) => (
          <div
            key={item.key}
            class={[
              'flex items-center justify-between p-4 transition-colors hover:bg-surface-container-low',
              index < settings.length - 1
                ? 'border-b border-outline-variant/50'
                : '',
            ]}
          >
            <div class="flex-1">
              <p class="text-sm font-medium leading-5 text-on-surface">
                {item.title}
              </p>
              <p class="text-sm leading-5 text-on-surface-variant">
                {item.desc}
              </p>
            </div>
            <Checkbox
              checked={item.checked}
              disabled={item.disabled}
              onChange={item.onChange}
            />
          </div>
        ))}
      </div>
    </div>
  );
});
