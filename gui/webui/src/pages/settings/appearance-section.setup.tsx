import { __render } from '@/shared/helpter';
import { saveAppLanguage } from '@/api/client';
import { Icon } from '@/components/icon';
import { Select } from '@/components/select';
import { toast } from '@/components/toast';
import { useClientQuery } from '@/hooks/use-client-query';
import { PaletteOutlined } from '@vicons/material';
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { i18n, setLocale, type SupportedLocale } from '@/i18n';
import { useRuntimeSettings } from '@/store/app';

export type ThemeMode = 'light' | 'dark' | 'auto';

export interface AppearanceSectionProps {}

defineProps<AppearanceSectionProps>();

const THEME_COOKIE_KEY = 'nsb-theme';
const themeMode = ref<ThemeMode>('light');
const requestedLocale = ref<SupportedLocale>(
  i18n.global.locale.value as SupportedLocale,
);
const runtimeSettings = useRuntimeSettings();

const appLanguageUpdateQuery = useClientQuery({
  queryKey: ['appLanguage', 'update'],
  enabled: false,
  queryFn: async () => {
    await saveAppLanguage(requestedLocale.value);
    const result = await runtimeSettings.refetch();
    if (!result.data) {
      throw new Error(i18n.global.t('errors.loadSettings'));
    }
    return result.data;
  },
});

function normalizeThemeMode(value: string | null | undefined): ThemeMode {
  return value === 'dark' || value === 'auto' ? value : 'light';
}

function readThemeModeFromCookie(): ThemeMode {
  if (typeof document === 'undefined') {
    return 'light';
  }

  const entry = document.cookie
    .split(';')
    .map((part) => part.trim())
    .find((part) => part.startsWith(`${THEME_COOKIE_KEY}=`));

  return normalizeThemeMode(entry?.split('=')[1]);
}

function applyThemeMode(value: ThemeMode) {
  if (typeof document === 'undefined') {
    return;
  }

  document.documentElement.setAttribute('data-nsb-theme', value);
}

function persistThemeMode(value: ThemeMode) {
  if (typeof document === 'undefined') {
    return;
  }

  document.cookie = `${THEME_COOKIE_KEY}=${value}; Path=/; Max-Age=31536000; SameSite=Lax`;
}

function selectThemeMode(value: ThemeMode) {
  themeMode.value = value;
  applyThemeMode(value);
  persistThemeMode(value);
}

async function selectLocale(locale: SupportedLocale) {
  if (
    appLanguageUpdateQuery.isFetching.value ||
    locale === i18n.global.locale.value
  ) {
    return;
  }

  requestedLocale.value = locale;
  try {
    await appLanguageUpdateQuery.refetch();
    setLocale(locale);
  } catch (error) {
    toast.error({
      content:
        error instanceof Error
          ? error.message
          : i18n.global.t('errors.saveSettings'),
      title: i18n.global.t('errors.saveSettingsTitle'),
    });
  }
}

onMounted(() => {
  themeMode.value = readThemeModeFromCookie();
  applyThemeMode(themeMode.value);
  const initialLocale = requestedLocale.value;
  void runtimeSettings.refetch().then((result) => {
    if (
      requestedLocale.value !== initialLocale ||
      appLanguageUpdateQuery.isFetching.value ||
      !result.data
    ) {
      return;
    }

    const appLanguage = result.data.app_language;
    requestedLocale.value = appLanguage;
    setLocale(appLanguage);
  });
});

const themeOptions = [
  {
    value: 'light' as ThemeMode,
    label: 'Light',
    selectedLabel: 'Sunlit',
    previewClass: 'bg-white',
    innerClass: 'bg-[#eef3ff]',
    topBarClass: 'bg-[#94a3b8]',
    midBarClass: 'bg-white',
    bodyClass: 'bg-[#dfe7fb]',
  },
  {
    value: 'dark' as ThemeMode,
    label: 'Dark',
    selectedLabel: 'Nocturne',
    previewClass: 'bg-[#0f172a]',
    innerClass: 'bg-[#1f2937]',
    topBarClass: 'bg-[#64748b]',
    midBarClass: 'bg-[#334155]',
    bodyClass: 'bg-[#5b6271]',
  },
  {
    value: 'auto' as ThemeMode,
    label: 'Auto',
    selectedLabel: 'System',
    previewClass:
      'bg-[linear-gradient(135deg,#f8fafc_0%,#f8fafc_48%,#0f172a_52%,#111827_100%)]',
    innerClass: '',
    topBarClass: '',
    midBarClass: '',
    bodyClass: '',
  },
];

defineOptions({ name: 'AppearanceSection' });

export default __render<AppearanceSectionProps>(() => <AppearanceContent />);

function AppearanceContent() {
  const { t } = useI18n();
  const locale = i18n.global.locale;
  const localeOptions = [
    { value: 'en-US', label: t('appearance.english') },
    { value: 'zh-CN', label: t('appearance.chinese') },
  ];
  return (
    <div class="mb-10">
      <div class="mb-3 flex items-center gap-2">
        <Icon class="text-xl text-primary">
          <PaletteOutlined />
        </Icon>
        <h3 class="text-lg font-bold leading-6 text-foreground">
          {t('appearance.title')}
        </h3>
      </div>
      <div class="rounded border border-outline-variant bg-surface-container-lowest p-4">
        <p class="mb-3 text-sm font-medium leading-5 text-foreground">
          {t('appearance.theme')}
        </p>
        <div class="grid grid-cols-3 gap-4">
          {themeOptions.map((item) => {
            const active = themeMode.value === item.value;
            return (
              <button
                key={item.value}
                class="group flex flex-col gap-2 text-left focus:outline-none"
                onClick={() => {
                  selectThemeMode(item.value);
                }}
              >
                <div
                  class={[
                    'aspect-video w-full cursor-pointer overflow-hidden rounded border-2 p-2 transition-all hover:-translate-y-0.5',
                    active
                      ? 'border-primary shadow-sm ring-2 ring-primary/10'
                      : 'border-outline',
                    item.previewClass,
                  ]}
                >
                  {item.value === 'auto' ? (
                    <div class="h-full w-full overflow-hidden rounded-sm p-1">
                      <div class="flex h-full w-full flex-col gap-1 rounded-sm bg-[rgba(255,255,255,0.12)] p-1">
                        <div class="h-1 w-1/2 rounded-full bg-[rgba(148,163,184,0.9)]"></div>
                        <div class="h-2 w-full rounded-sm bg-[linear-gradient(90deg,rgba(255,255,255,0.92)_0%,rgba(255,255,255,0.92)_48%,rgba(51,65,85,0.92)_52%,rgba(51,65,85,0.92)_100%)]"></div>
                        <div class="flex-1 rounded-sm bg-[linear-gradient(180deg,#dbeafe_0%,#dbeafe_48%,#475569_52%,#334155_100%)]"></div>
                      </div>
                    </div>
                  ) : (
                    <div
                      class={[
                        'flex h-full w-full flex-col gap-1 rounded-sm p-1',
                        item.innerClass,
                      ]}
                    >
                      <div
                        class={['h-1 w-1/2 rounded-full', item.topBarClass]}
                      ></div>
                      <div
                        class={['h-2 w-full rounded-sm', item.midBarClass]}
                      ></div>
                      <div class={['flex-1 rounded-sm', item.bodyClass]}></div>
                    </div>
                  )}
                </div>
                <span
                  class={[
                    'w-full text-center text-xs font-medium',
                    active ? 'text-primary' : 'text-muted-fg',
                  ]}
                >
                  {t(
                    `appearance.${active ? item.selectedLabel.toLowerCase() : item.label.toLowerCase()}`,
                  )}
                </span>
              </button>
            );
          })}
        </div>
        <div class="mt-6 flex h-11 items-center justify-between gap-4 border-t border-outline-variant pt-4 text-sm text-on-surface">
          <span class="font-medium">{t('appearance.language')}</span>
          <Select
            ariaLabel={t('appearance.language')}
            disabled={appLanguageUpdateQuery.isFetching.value}
            modelValue={locale.value}
            options={localeOptions}
            onUpdateModelValue={(value) => {
              void selectLocale(value as SupportedLocale);
            }}
          />
        </div>
      </div>
    </div>
  );
}
